use std::sync::Arc;

use tokio::sync::broadcast;
use tracing::info;
use uuid::Uuid;

use gt_common::protocol::{
    AuthRequest, AuthResponse, ClientMessage, ErrorCode, ServerMessage,
};

use crate::auth;
use crate::state::{AppState, ConnectedPlayer};

pub(crate) async fn handle_client_message(
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

            Some(
                super::sync::handle_join_world(
                    world_id,
                    state,
                    p,
                    world_broadcast_rx,
                    forward_tx,
                )
                .await,
            )
        }

        ClientMessage::LeaveWorld => {
            if let Some(p) = player {
                Some(super::sync::handle_leave_world(state, p).await)
            } else {
                Some(ServerMessage::CommandAck {
                    success: true,
                    error: None,
                    seq: None,
                    entity_id: None,
                    effective_tick: None,
                })
            }
        }

        ClientMessage::GameCommand {
            world_id,
            command,
            seq,
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

            Some(
                super::commands::handle_game_command(world_id, command, seq, state, p).await,
            )
        }

        ClientMessage::RequestSnapshot { world_id } => {
            Some(super::sync::handle_request_snapshot(world_id, state).await)
        }

        ClientMessage::Ping { timestamp } => Some(super::sync::handle_ping(timestamp)),

        ClientMessage::Chat { message } => {
            let p = match player {
                Some(p) => p,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::NotAuthenticated,
                        message: "Not authenticated".to_string(),
                    });
                }
            };

            super::sync::handle_chat(message, state, p).await
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

            Some(
                super::sync::handle_upload_save(slot, name, save_data, tick, config_json, state, p)
                    .await,
            )
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

            Some(super::sync::handle_request_saves(state, p).await)
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

            Some(super::sync::handle_download_save(slot, state, p).await)
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

            Some(super::sync::handle_delete_save(slot, state, p).await)
        }

        ClientMessage::InviteFriend {
            friend_id,
            world_id,
        } => {
            let p = match player {
                Some(p) => p,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::NotAuthenticated,
                        message: "Must authenticate first".to_string(),
                    });
                }
            };

            Some(super::sync::handle_invite_friend(friend_id, world_id, state, p).await)
        }
    }
}

pub(crate) async fn handle_auth(
    req: AuthRequest,
    state: &Arc<AppState>,
    player: &mut Option<ConnectedPlayer>,
) -> ServerMessage {
    match req {
        AuthRequest::Login {
            username,
            password,
            spectator,
        } => {
            let account = match state.get_account(&username).await {
                Some(a) => a,
                None => {
                    return ServerMessage::AuthResult(AuthResponse::Failed {
                        reason: "Invalid credentials: account not found".to_string(),
                    });
                }
            };

            match auth::verify_password(&password, &account.password_hash) {
                Ok(true) => {}
                Ok(false) => {
                    return ServerMessage::AuthResult(AuthResponse::Failed {
                        reason: "Invalid credentials: incorrect password".to_string(),
                    });
                }
                Err(_) => {
                    return ServerMessage::AuthResult(AuthResponse::Failed {
                        reason: "Invalid credentials: password verification error".to_string(),
                    });
                }
            }

            // Check if this player is already connected (reconnection scenario)
            let existing_session = state.players.read().await.get(&account.id).cloned();
            if let Some(ref existing) = existing_session {
                info!(
                    "Player {} reconnecting (was in world {:?})",
                    account.username, existing.world_id
                );
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
                is_spectator: spectator,
                world_id: existing_session.as_ref().and_then(|s| s.world_id),
                corp_id: if spectator {
                    None
                } else {
                    existing_session.as_ref().and_then(|s| s.corp_id)
                },
            };

            state
                .players
                .write()
                .await
                .insert(account.id, connected.clone());
            *player = Some(connected);

            // Register online presence
            register_presence(
                state,
                account.id,
                &account.username,
                &account.display_name,
                &account.avatar_id,
            )
            .await;

            if spectator {
                info!("Spectator {} logged in", account.username);
            } else {
                info!("Player {} logged in", account.username);
            }

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
            spectator,
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
                        is_spectator: spectator,
                        world_id: None,
                        corp_id: None,
                    };

                    state
                        .players
                        .write()
                        .await
                        .insert(account.id, connected.clone());
                    *player = Some(connected);

                    // Register online presence
                    register_presence(
                        state,
                        account.id,
                        &account.username,
                        &account.display_name,
                        &account.avatar_id,
                    )
                    .await;

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
        AuthRequest::Token {
            access_token,
            spectator,
        } => {
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

                    // Restore previous session state if reconnecting
                    let existing_session = state.players.read().await.get(&player_id).cloned();
                    if let Some(ref existing) = existing_session {
                        info!(
                            "Player {} resuming session via token (was in world {:?})",
                            claims.username, existing.world_id
                        );
                    }

                    let connected = ConnectedPlayer {
                        id: player_id,
                        username: claims.username.clone(),
                        is_guest: claims.is_guest,
                        is_admin: false,
                        is_spectator: spectator,
                        world_id: existing_session.as_ref().and_then(|s| s.world_id),
                        corp_id: if spectator {
                            None
                        } else {
                            existing_session.as_ref().and_then(|s| s.corp_id)
                        },
                    };

                    state
                        .players
                        .write()
                        .await
                        .insert(player_id, connected.clone());
                    *player = Some(connected);

                    // Register online presence
                    register_presence(
                        state,
                        player_id,
                        &claims.username,
                        &None,
                        "tower_01",
                    )
                    .await;

                    info!("Player {} resumed session via token", claims.username);

                    ServerMessage::AuthResult(AuthResponse::Success {
                        player_id,
                        username: claims.username,
                        access_token,
                        refresh_token: String::new(),
                    })
                }
                Err(e) => ServerMessage::AuthResult(AuthResponse::Failed {
                    reason: format!("Invalid or expired token: {e}"),
                }),
            }
        }
        AuthRequest::TokenRefresh { refresh_token } => {
            match auth::validate_token(&state.auth_config, &refresh_token) {
                Ok(claims) => {
                    let player_id =
                        Uuid::parse_str(&claims.sub).unwrap_or_else(|_| Uuid::new_v4());
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
                    reason: "Invalid or expired refresh token".to_string(),
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
                is_spectator: false,
                world_id: None,
                corp_id: None,
            };

            state
                .players
                .write()
                .await
                .insert(account.id, connected.clone());
            *player = Some(connected);

            // Register online presence
            register_presence(state, account.id, &account.username, &None, "tower_01").await;

            info!("Guest player joined: {}", account.username);

            ServerMessage::AuthResult(AuthResponse::GuestSuccess {
                player_id: account.id,
                username: account.username,
            })
        }
    }
}

/// Register a player's online presence and notify friends
pub(crate) async fn register_presence(
    state: &Arc<AppState>,
    player_id: Uuid,
    username: &str,
    display_name: &Option<String>,
    avatar_id: &str,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    state.online_players.write().await.insert(
        player_id,
        crate::state::OnlinePresence {
            username: username.to_string(),
            display_name: display_name.clone(),
            avatar_id: avatar_id.to_string(),
            world_id: None,
            world_name: None,
            connected_at: now,
        },
    );

    // Notify friends that this player came online
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        if let Ok(friends) = db.list_friends(player_id).await {
            let msg = ServerMessage::FriendPresenceUpdate {
                friend_id: player_id,
                username: username.to_string(),
                online: true,
                world_id: None,
                world_name: None,
            };
            // Send to each online friend via their world's broadcast channel
            let online = state.online_players.read().await;
            for friend in &friends {
                if let Some(presence) = online.get(&friend.friend_id) {
                    if let Some(wid) = presence.world_id {
                        if let Some(world) = state.worlds.read().await.get(&wid) {
                            let _ = world.broadcast_tx.send(msg.clone());
                        }
                    }
                }
            }
        }
    }
}

/// Remove a player's online presence and notify friends
pub(crate) async fn unregister_presence(state: &Arc<AppState>, player_id: Uuid, username: &str) {
    state.online_players.write().await.remove(&player_id);

    // Notify friends that this player went offline
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        if let Ok(friends) = db.list_friends(player_id).await {
            let msg = ServerMessage::FriendPresenceUpdate {
                friend_id: player_id,
                username: username.to_string(),
                online: false,
                world_id: None,
                world_name: None,
            };
            let online = state.online_players.read().await;
            for friend in &friends {
                if let Some(presence) = online.get(&friend.friend_id) {
                    if let Some(wid) = presence.world_id {
                        if let Some(world) = state.worlds.read().await.get(&wid) {
                            let _ = world.broadcast_tx.send(msg.clone());
                        }
                    }
                }
            }
        }
    }
}
