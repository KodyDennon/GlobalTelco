use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid::Uuid;

use gt_common::protocol::{
    deserialize_msgpack, serialize_msgpack, AuthRequest, AuthResponse, ClientMessage, ErrorCode,
    PlayerConnectionStatus, ServerMessage,
};

use crate::auth;
use crate::state::{AppState, ConnectedPlayer};

/// Handle an individual WebSocket connection
pub async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    let mut player: Option<ConnectedPlayer> = None;
    let mut world_broadcast_rx: Option<broadcast::Receiver<ServerMessage>> = None;

    // Spawn a task to forward broadcast messages to this client
    let (forward_tx, mut forward_rx) = tokio::sync::mpsc::channel::<ServerMessage>(64);

    // Forward task: sends broadcast messages to the WebSocket
    let forward_sender = tokio::spawn(async move {
        while let Some(msg) = forward_rx.recv().await {
            if let Ok(bytes) = serialize_msgpack(&msg) {
                if sender.send(Message::Binary(bytes.into())).await.is_err() {
                    break;
                }
            }
        }
        sender
    });

    // Main receive loop
    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(Message::Binary(data)) => match deserialize_msgpack::<ClientMessage>(&data) {
                Ok(m) => m,
                Err(e) => {
                    warn!("Failed to deserialize client message: {e}");
                    continue;
                }
            },
            Ok(Message::Text(text)) => {
                // Also accept JSON for debug/development
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(m) => m,
                    Err(e) => {
                        warn!("Failed to deserialize JSON client message: {e}");
                        continue;
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(_) | Message::Pong(_)) => continue,
            Err(e) => {
                error!("WebSocket error: {e}");
                break;
            }
        };

        let response = handle_client_message(
            msg,
            &state,
            &mut player,
            &mut world_broadcast_rx,
            &forward_tx,
        )
        .await;

        if let Some(resp) = response {
            if forward_tx.send(resp).await.is_err() {
                break;
            }
        }
    }

    // Cleanup: remove player from world and connected players
    if let Some(p) = &player {
        info!("Player {} disconnected", p.username);

        if let Some(world_id) = &p.world_id {
            if let Some(world) = state.get_world(world_id).await {
                world.remove_player(&p.id).await;

                // Notify other players
                let _ = world.broadcast_tx.send(ServerMessage::PlayerStatus {
                    player_id: p.id,
                    username: p.username.clone(),
                    status: PlayerConnectionStatus::Disconnected,
                });
            }
        }

        state.players.write().await.remove(&p.id);
    }

    // Wait for forward task to finish
    let _ = forward_sender.await;
}

async fn handle_client_message(
    msg: ClientMessage,
    state: &Arc<AppState>,
    player: &mut Option<ConnectedPlayer>,
    world_broadcast_rx: &mut Option<broadcast::Receiver<ServerMessage>>,
    forward_tx: &tokio::sync::mpsc::Sender<ServerMessage>,
) -> Option<ServerMessage> {
    match msg {
        ClientMessage::Auth(auth_req) => Some(handle_auth(auth_req, state, player).await),

        ClientMessage::JoinWorld { world_id } => {
            let p = match player {
                Some(p) => p,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::NotAuthenticated,
                        message: "Must authenticate first".to_string(),
                    });
                }
            };

            let world = match state.get_world(&world_id).await {
                Some(w) => w,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::WorldNotFound,
                        message: "World not found".to_string(),
                    });
                }
            };

            if world.is_full().await {
                return Some(ServerMessage::Error {
                    code: ErrorCode::WorldFull,
                    message: "World is full".to_string(),
                });
            }

            // Create a corporation for this player in the world
            let corp_id = {
                let mut w = world.world.lock().await;
                w.allocate_entity()
            };

            // Add player to world
            world.add_player(p.id, corp_id).await;
            p.world_id = Some(world_id);
            p.corp_id = Some(corp_id);

            // Subscribe to world broadcasts
            *world_broadcast_rx = Some(world.broadcast_tx.subscribe());

            // Spawn broadcast forwarder
            if let Some(mut rx) = world_broadcast_rx.take() {
                let tx = forward_tx.clone();
                tokio::spawn(async move {
                    while let Ok(msg) = rx.recv().await {
                        if tx.send(msg).await.is_err() {
                            break;
                        }
                    }
                });
                *world_broadcast_rx = Some(world.broadcast_tx.subscribe());
            }

            // Notify other players
            let _ = world.broadcast_tx.send(ServerMessage::PlayerStatus {
                player_id: p.id,
                username: p.username.clone(),
                status: PlayerConnectionStatus::Connected,
            });

            let tick = world.world.lock().await.current_tick();

            info!(
                "Player {} joined world {} as corp {}",
                p.username, world_id, corp_id
            );

            Some(ServerMessage::WorldJoined {
                world_id,
                corp_id,
                tick,
            })
        }

        ClientMessage::LeaveWorld => {
            if let Some(p) = player {
                if let Some(world_id) = p.world_id.take() {
                    if let Some(world) = state.get_world(&world_id).await {
                        world.remove_player(&p.id).await;
                        let _ = world.broadcast_tx.send(ServerMessage::PlayerStatus {
                            player_id: p.id,
                            username: p.username.clone(),
                            status: PlayerConnectionStatus::Disconnected,
                        });
                    }
                }
                p.corp_id = None;
            }
            Some(ServerMessage::CommandAck {
                success: true,
                error: None,
            })
        }

        ClientMessage::GameCommand { world_id, command } => {
            let p = match player {
                Some(p) => p,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::NotAuthenticated,
                        message: "Not authenticated".to_string(),
                    });
                }
            };

            if p.world_id.as_ref() != Some(&world_id) {
                return Some(ServerMessage::Error {
                    code: ErrorCode::NotInWorld,
                    message: "Not in this world".to_string(),
                });
            }

            let world = match state.get_world(&world_id).await {
                Some(w) => w,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::WorldNotFound,
                        message: "World not found".to_string(),
                    });
                }
            };

            // Process the command
            let mut w = world.world.lock().await;
            w.process_command(command);

            Some(ServerMessage::CommandAck {
                success: true,
                error: None,
            })
        }

        ClientMessage::RequestSnapshot { world_id } => {
            let world = match state.get_world(&world_id).await {
                Some(w) => w,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::WorldNotFound,
                        message: "World not found".to_string(),
                    });
                }
            };

            let w = world.world.lock().await;
            let tick = w.current_tick();
            // Serialize a summary of the world state
            let state_json = serde_json::to_string(&serde_json::json!({
                "tick": tick,
                "config": world.config,
            }))
            .unwrap_or_default();

            Some(ServerMessage::Snapshot { tick, state_json })
        }

        ClientMessage::Ping { timestamp } => {
            let server_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            Some(ServerMessage::Pong {
                timestamp,
                server_time,
            })
        }

        ClientMessage::Chat { message } => {
            if let Some(p) = player {
                if let Some(world_id) = &p.world_id {
                    if let Some(world) = state.get_world(world_id).await {
                        let _ = world.broadcast_tx.send(ServerMessage::ChatBroadcast {
                            sender: p.username.clone(),
                            message,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                        });
                    }
                }
            }
            None // No direct response for chat, it's broadcast
        }
    }
}

async fn handle_auth(
    req: AuthRequest,
    state: &Arc<AppState>,
    player: &mut Option<ConnectedPlayer>,
) -> ServerMessage {
    match req {
        AuthRequest::Login { username, password } => {
            let account = match state.get_account(&username).await {
                Some(a) => a,
                None => {
                    return ServerMessage::AuthResult(AuthResponse::Failed {
                        reason: "Invalid credentials".to_string(),
                    });
                }
            };

            match auth::verify_password(&password, &account.password_hash) {
                Ok(true) => {}
                _ => {
                    return ServerMessage::AuthResult(AuthResponse::Failed {
                        reason: "Invalid credentials".to_string(),
                    });
                }
            }

            let access_token = auth::generate_access_token(
                &state.auth_config,
                account.id,
                &account.username,
                false,
            )
            .unwrap_or_default();

            let refresh_token =
                auth::generate_refresh_token(&state.auth_config, account.id, &account.username)
                    .unwrap_or_default();

            let connected = ConnectedPlayer {
                id: account.id,
                username: account.username.clone(),
                is_guest: false,
                world_id: None,
                corp_id: None,
            };

            state
                .players
                .write()
                .await
                .insert(account.id, connected.clone());
            *player = Some(connected);

            info!("Player {} logged in", account.username);

            ServerMessage::AuthResult(AuthResponse::Success {
                player_id: account.id,
                username: account.username,
                access_token,
                refresh_token,
            })
        }

        AuthRequest::Register {
            username,
            password,
            email,
        } => {
            let password_hash = match auth::hash_password(&password) {
                Ok(h) => h,
                Err(e) => {
                    return ServerMessage::AuthResult(AuthResponse::Failed {
                        reason: format!("Password hashing failed: {e}"),
                    });
                }
            };

            match state
                .register_account(username, Some(email), password_hash)
                .await
            {
                Ok(account) => {
                    let access_token = auth::generate_access_token(
                        &state.auth_config,
                        account.id,
                        &account.username,
                        false,
                    )
                    .unwrap_or_default();

                    let refresh_token = auth::generate_refresh_token(
                        &state.auth_config,
                        account.id,
                        &account.username,
                    )
                    .unwrap_or_default();

                    let connected = ConnectedPlayer {
                        id: account.id,
                        username: account.username.clone(),
                        is_guest: false,
                        world_id: None,
                        corp_id: None,
                    };

                    state
                        .players
                        .write()
                        .await
                        .insert(account.id, connected.clone());
                    *player = Some(connected);

                    info!("New account registered: {}", account.username);

                    ServerMessage::AuthResult(AuthResponse::Success {
                        player_id: account.id,
                        username: account.username,
                        access_token,
                        refresh_token,
                    })
                }
                Err(e) => ServerMessage::AuthResult(AuthResponse::Failed { reason: e }),
            }
        }

        AuthRequest::TokenRefresh { refresh_token } => {
            match auth::validate_token(&state.auth_config, &refresh_token) {
                Ok(claims) => {
                    let player_id = Uuid::parse_str(&claims.sub).unwrap_or_else(|_| Uuid::new_v4());
                    let access_token = auth::generate_access_token(
                        &state.auth_config,
                        player_id,
                        &claims.username,
                        claims.is_guest,
                    )
                    .unwrap_or_default();

                    let new_refresh = auth::generate_refresh_token(
                        &state.auth_config,
                        player_id,
                        &claims.username,
                    )
                    .unwrap_or_default();

                    ServerMessage::AuthResult(AuthResponse::Success {
                        player_id,
                        username: claims.username,
                        access_token,
                        refresh_token: new_refresh,
                    })
                }
                Err(_) => ServerMessage::AuthResult(AuthResponse::Failed {
                    reason: "Invalid refresh token".to_string(),
                }),
            }
        }

        AuthRequest::Guest => {
            let account = state.register_guest().await;

            let connected = ConnectedPlayer {
                id: account.id,
                username: account.username.clone(),
                is_guest: true,
                world_id: None,
                corp_id: None,
            };

            state
                .players
                .write()
                .await
                .insert(account.id, connected.clone());
            *player = Some(connected);

            info!("Guest player joined: {}", account.username);

            ServerMessage::AuthResult(AuthResponse::GuestSuccess {
                player_id: account.id,
                username: account.username,
            })
        }
    }
}
