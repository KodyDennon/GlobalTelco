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

use gt_common::commands::Command;

use crate::auth;
use crate::state::{AppState, ConnectedPlayer};

/// Extract the corporation EntityId that a command targets, if any.
/// Used for anti-cheat validation: ensures players can only issue commands
/// that affect their own corporation.
fn command_target_corp(command: &Command) -> Option<gt_common::types::EntityId> {
    match command {
        Command::HireEmployee { corporation, .. }
        | Command::TakeLoan { corporation, .. }
        | Command::SetBudget { corporation, .. }
        | Command::StartResearch { corporation, .. }
        | Command::CancelResearch { corporation }
        | Command::SetPolicy { corporation, .. }
        | Command::DeclareBankruptcy { entity: corporation }
        | Command::RequestBailout { entity: corporation }
        | Command::AcceptBailout { entity: corporation } => Some(*corporation),
        Command::ProposeContract { from, .. } => Some(*from),
        Command::CreateSubsidiary { parent, .. } => Some(*parent),
        // Commands that operate on entities (nodes, edges, etc.) rather than
        // directly referencing a corp -- ownership is checked inside the
        // simulation engine, so we skip corp-level gating here.
        Command::BuildNode { .. }
        | Command::BuildEdge { .. }
        | Command::UpgradeNode { .. }
        | Command::DecommissionNode { .. }
        | Command::RepairNode { .. }
        | Command::EmergencyRepair { .. }
        | Command::FireEmployee { .. }
        | Command::AssignTeam { .. }
        | Command::RepayLoan { .. }
        | Command::AcceptContract { .. }
        | Command::RejectContract { .. }
        | Command::PurchaseInsurance { .. }
        | Command::CancelInsurance { .. }
        | Command::PlaceBid { .. }
        | Command::ProposeAcquisition { .. }
        | Command::RespondToAcquisition { .. }
        | Command::LaunchEspionage { .. }
        | Command::LaunchSabotage { .. }
        | Command::UpgradeSecurity { .. }
        | Command::StartLobbying { .. }
        | Command::CancelLobbying { .. }
        | Command::ProposeCoOwnership { .. }
        | Command::RespondCoOwnership { .. }
        | Command::ProposeBuyout { .. }
        | Command::VoteUpgrade { .. }
        | Command::SetSpeed(_)
        | Command::TogglePause
        | Command::SaveGame { .. }
        | Command::LoadGame { .. } => None,
    }
}

/// Simple sliding window rate limiter
struct RateLimiter {
    command_timestamps: Vec<std::time::Instant>,
    chat_timestamps: Vec<std::time::Instant>,
    max_commands_per_sec: usize,
    max_chats_per_10sec: usize,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            command_timestamps: Vec::new(),
            chat_timestamps: Vec::new(),
            max_commands_per_sec: 10,
            max_chats_per_10sec: 5,
        }
    }

    fn check_command(&mut self) -> bool {
        let now = std::time::Instant::now();
        let cutoff = now - std::time::Duration::from_secs(1);
        self.command_timestamps.retain(|t| *t > cutoff);
        if self.command_timestamps.len() >= self.max_commands_per_sec {
            return false;
        }
        self.command_timestamps.push(now);
        true
    }

    fn check_chat(&mut self) -> bool {
        let now = std::time::Instant::now();
        let cutoff = now - std::time::Duration::from_secs(10);
        self.chat_timestamps.retain(|t| *t > cutoff);
        if self.chat_timestamps.len() >= self.max_chats_per_10sec {
            return false;
        }
        self.chat_timestamps.push(now);
        true
    }
}

/// Handle an individual WebSocket connection
pub async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    let mut player: Option<ConnectedPlayer> = None;
    let mut world_broadcast_rx: Option<broadcast::Receiver<ServerMessage>> = None;
    let mut rate_limiter = RateLimiter::new();

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

        // Rate limit commands and chat
        let is_rate_limited = match &msg {
            ClientMessage::GameCommand { .. }
            | ClientMessage::UploadSave { .. }
            | ClientMessage::DeleteSave { .. } => !rate_limiter.check_command(),
            ClientMessage::Chat { .. } => !rate_limiter.check_chat(),
            _ => false,
        };

        if is_rate_limited {
            let _ = forward_tx
                .send(ServerMessage::Error {
                    code: ErrorCode::RateLimited,
                    message: "Too many requests, slow down".to_string(),
                })
                .await;
            continue;
        }

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

    // Cleanup: activate AI proxy for disconnected player's corporation
    if let Some(p) = &player {
        info!("Player {} disconnected", p.username);

        if let Some(world_id) = &p.world_id {
            if let Some(world) = state.get_world(world_id).await {
                // Activate AI proxy for this player's corporation
                if let Some(corp_id) = p.corp_id {
                    let mut w = world.world.lock().await;
                    // Create a defensive AI proxy for the disconnected player
                    let proxy_ai = gt_simulation::components::ai_state::AiState {
                        archetype: gt_common::types::AIArchetype::DefensiveConsolidator,
                        strategy: gt_common::types::AIStrategy::Consolidate,
                        aggression: 0.2,
                        risk_tolerance: 0.1,
                        proxy_mode: true,
                    };
                    w.ai_states.insert(corp_id, proxy_ai);
                    info!(
                        "AI proxy activated for corp {} (player {})",
                        corp_id, p.username
                    );
                }

                // Don't remove the player from the world — they're just disconnected
                // The AI proxy will manage their corp until they reconnect

                // Update database session if available
                #[cfg(feature = "postgres")]
                if let Some(db) = state.db.as_ref() {
                    let _ = db.set_player_disconnected(p.id, *world_id).await;
                }

                // Notify other players of AI proxy status
                let _ = world.broadcast_tx.send(ServerMessage::PlayerStatus {
                    player_id: p.id,
                    username: p.username.clone(),
                    status: PlayerConnectionStatus::AiProxy,
                });
            }
        }

        state.players.write().await.remove(&p.id);
    }

    // Wait for forward task to finish
    let _ = forward_sender.await;
}

#[allow(unused_variables)]
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

            // Check if this player already has a corp in this world (reconnection)
            let (corp_id, proxy_ticks, proxy_was_active) = {
                let existing_players = world.players.read().await;
                if let Some(&existing_corp) = existing_players.get(&p.id) {
                    // Reconnecting — deactivate AI proxy and build summary
                    let mut w = world.world.lock().await;
                    let was_proxy = w
                        .ai_states
                        .get(&existing_corp)
                        .map(|ai| ai.proxy_mode)
                        .unwrap_or(false);
                    let ticks = w.current_tick();
                    // Remove AI proxy
                    w.ai_states.remove(&existing_corp);

                    #[cfg(feature = "postgres")]
                    if let Some(db) = state.db.as_ref() {
                        let _ = db.set_player_connected(p.id, world_id).await;
                    }

                    (existing_corp, ticks, was_proxy)
                } else {
                    drop(existing_players);
                    // New player — create corporation
                    let mut w = world.world.lock().await;
                    let new_corp = w.allocate_entity();
                    (new_corp, 0, false)
                }
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

            // Send proxy summary first if reconnecting from AI proxy
            if proxy_was_active {
                let _ = forward_tx
                    .send(ServerMessage::ProxySummary {
                        ticks_elapsed: proxy_ticks,
                        actions: vec![gt_common::protocol::ProxyAction {
                            tick: proxy_ticks,
                            description:
                                "AI proxy maintained your corporation while you were away"
                                    .to_string(),
                        }],
                    })
                    .await;
            }

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

            // Anti-cheat: verify the player owns the corporation targeted by the command
            if let Some(target_corp) = command_target_corp(&command) {
                if p.corp_id != Some(target_corp) {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::PermissionDenied,
                        message: "Command targets a corporation you do not own".to_string(),
                    });
                }
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

            // Process the command and log it
            let mut w = world.world.lock().await;
            let tick = w.current_tick();
            let command_debug = format!("{:?}", command);
            w.process_command(command);
            drop(w);

            state.log_command(p.id, command_debug, tick).await;

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

        ClientMessage::UploadSave {
            slot,
            name,
            save_data,
            tick,
            config_json,
        } => {
            let p = match player {
                Some(p) => p,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::NotAuthenticated,
                        message: "Not authenticated".to_string(),
                    });
                }
            };

            #[cfg(feature = "postgres")]
            if let Some(db) = state.db.as_ref() {
                let config: serde_json::Value =
                    serde_json::from_str(&config_json).unwrap_or(serde_json::json!({}));
                match db
                    .save_cloud(p.id, slot, &name, &save_data, tick as i64, &config)
                    .await
                {
                    Ok(_) => {
                        return Some(ServerMessage::SaveUploaded {
                            slot,
                            success: true,
                        });
                    }
                    Err(e) => {
                        warn!("Cloud save upload failed: {e}");
                        return Some(ServerMessage::SaveUploaded {
                            slot,
                            success: false,
                        });
                    }
                }
            }

            Some(ServerMessage::Error {
                code: ErrorCode::InternalError,
                message: "Database not available".to_string(),
            })
        }

        ClientMessage::RequestSaves => {
            let p = match player {
                Some(p) => p,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::NotAuthenticated,
                        message: "Not authenticated".to_string(),
                    });
                }
            };

            #[cfg(feature = "postgres")]
            if let Some(db) = state.db.as_ref() {
                match db.list_cloud_saves(p.id).await {
                    Ok(rows) => {
                        let saves = rows
                            .into_iter()
                            .map(|r| gt_common::protocol::CloudSaveInfo {
                                slot: r.slot,
                                name: r.name,
                                tick: r.tick as u64,
                                size_bytes: r.size_bytes,
                                created_at: r.created_at.timestamp() as u64,
                            })
                            .collect();
                        return Some(ServerMessage::SaveList { saves });
                    }
                    Err(e) => {
                        warn!("Failed to list cloud saves: {e}");
                    }
                }
            }

            Some(ServerMessage::SaveList { saves: vec![] })
        }

        ClientMessage::DownloadSave { slot } => {
            let p = match player {
                Some(p) => p,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::NotAuthenticated,
                        message: "Not authenticated".to_string(),
                    });
                }
            };

            #[cfg(feature = "postgres")]
            if let Some(db) = state.db.as_ref() {
                match db.load_cloud_save(p.id, slot).await {
                    Ok(Some(data)) => {
                        return Some(ServerMessage::SaveData {
                            slot,
                            save_data: data,
                        });
                    }
                    Ok(None) => {
                        return Some(ServerMessage::Error {
                            code: ErrorCode::InvalidCommand,
                            message: "Save not found".to_string(),
                        });
                    }
                    Err(e) => {
                        warn!("Failed to load cloud save: {e}");
                    }
                }
            }

            Some(ServerMessage::Error {
                code: ErrorCode::InternalError,
                message: "Database not available".to_string(),
            })
        }

        ClientMessage::DeleteSave { slot } => {
            let p = match player {
                Some(p) => p,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::NotAuthenticated,
                        message: "Not authenticated".to_string(),
                    });
                }
            };

            #[cfg(feature = "postgres")]
            if let Some(db) = state.db.as_ref() {
                match db.delete_cloud_save(p.id, slot).await {
                    Ok(deleted) => {
                        return Some(ServerMessage::CommandAck {
                            success: deleted,
                            error: if deleted {
                                None
                            } else {
                                Some("Save not found".to_string())
                            },
                        });
                    }
                    Err(e) => {
                        warn!("Failed to delete cloud save: {e}");
                    }
                }
            }

            Some(ServerMessage::Error {
                code: ErrorCode::InternalError,
                message: "Database not available".to_string(),
            })
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
                is_admin: false,
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
                        is_admin: false,
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
        AuthRequest::Token { access_token } => {
            match auth::validate_token(&state.auth_config, &access_token) {
                Ok(claims) => {
                    let player_id = match Uuid::parse_str(&claims.sub) {
                        Ok(id) => id,
                        Err(_) => {
                            return ServerMessage::AuthResult(AuthResponse::Failed {
                                reason: "Invalid player ID in token".to_string(),
                            });
                        }
                    };

                    let connected = ConnectedPlayer {
                        id: player_id,
                        username: claims.username.clone(),
                        is_guest: claims.is_guest,
                        is_admin: false,
                        world_id: None,
                        corp_id: None,
                    };

                    state
                        .players
                        .write()
                        .await
                        .insert(player_id, connected.clone());
                    *player = Some(connected);

                    info!("Player {} resumed session via token", claims.username);

                    ServerMessage::AuthResult(AuthResponse::Success {
                        player_id,
                        username: claims.username,
                        access_token,
                        refresh_token: String::new(),
                    })
                }
                Err(e) => ServerMessage::AuthResult(AuthResponse::Failed {
                    reason: format!("Invalid token: {e}"),
                }),
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
                is_admin: false,
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
