use std::sync::Arc;

use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid::Uuid;

use gt_common::commands::Command;
use gt_common::protocol::{
    AuthRequest, AuthResponse, ClientMessage, ErrorCode, PlayerConnectionStatus, ServerMessage,
};

use crate::auth;
use crate::state::{AppState, ConnectedPlayer};

use super::chat::sanitize_chat;
use super::filtering::filter_tick_update_for_player;
use super::{MAX_CHAT_LENGTH, MAX_SAVE_SIZE};

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

            let world = match state.get_world(&world_id).await {
                Some(w) => w,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::WorldNotFound,
                        message: "World not found".to_string(),
                    });
                }
            };

            // Check if player is banned from this world (in-memory first, then DB)
            if world.banned_players.read().await.contains(&p.id) {
                return Some(ServerMessage::Error {
                    code: ErrorCode::PermissionDenied,
                    message: "You are banned from this world".to_string(),
                });
            }
            #[cfg(feature = "postgres")]
            if let Some(db) = state.db.as_ref() {
                if let Ok(true) = db.is_banned(p.id, Some(world_id)).await {
                    // Also add to in-memory set for future fast checks
                    world.banned_players.write().await.insert(p.id);
                    return Some(ServerMessage::Error {
                        code: ErrorCode::PermissionDenied,
                        message: "You are banned from this world".to_string(),
                    });
                }
            }

            // Spectators don't count toward the player limit
            if !p.is_spectator && world.is_full().await {
                return Some(ServerMessage::Error {
                    code: ErrorCode::WorldFull,
                    message: "World is full".to_string(),
                });
            }

            // Spectators don't get a corporation or participate in the world
            let (corp_id, proxy_ticks, proxy_was_active) = if p.is_spectator {
                // Spectator: no corp, no reconnection logic
                (0u64, 0u64, false)
            } else {
                // Check if this player already has a corp in this world (reconnection)
                let existing_players = world.players.read().await;
                if let Some(&existing_corp) = existing_players.get(&p.id) {
                    // Reconnecting -- deactivate AI proxy and build summary
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

                    info!(
                        "Player {} reconnected to world {} (corp {}, proxy_was_active: {})",
                        p.username, world_id, existing_corp, was_proxy
                    );

                    (existing_corp, ticks, was_proxy)
                } else {
                    drop(existing_players);
                    // New player -- create corporation with full components
                    let mut w = world.world.lock().await;
                    let new_corp = w.create_player_corporation(&p.username);
                    (new_corp, 0, false)
                }
            };

            if !p.is_spectator {
                // Add player to world's player map
                world.add_player(p.id, corp_id).await;

                // First non-spectator player becomes the creator (has speed override)
                let mut creator = world.creator_id.write().await;
                if creator.is_none() {
                    *creator = Some(p.id);
                }
                drop(creator);
            }

            p.world_id = Some(world_id);
            if !p.is_spectator {
                p.corp_id = Some(corp_id);
            }

            // Subscribe to world broadcasts with per-player filtering
            *world_broadcast_rx = Some(world.broadcast_tx.subscribe());

            if let Some(mut rx) = world_broadcast_rx.take() {
                let tx = forward_tx.clone();
                let player_corp_id = p.corp_id;
                let is_spectator = p.is_spectator;
                let world_ref = Arc::clone(&world);
                tokio::spawn(async move {
                    while let Ok(msg) = rx.recv().await {
                        // Snapshot this player's intel levels from the world state.
                        // The lock is held only for the brief HashMap scan (microseconds).
                        let intel = if let Some(my_corp) = player_corp_id {
                            if !is_spectator {
                                let w = world_ref.world.lock().await;
                                w.get_intel_levels_for_corp(my_corp)
                            } else {
                                std::collections::HashMap::new()
                            }
                        } else {
                            std::collections::HashMap::new()
                        };

                        // Apply per-player graduated data visibility filtering
                        let filtered =
                            filter_tick_update_for_player(&msg, player_corp_id, is_spectator, &intel);
                        if tx.send(filtered).await.is_err() {
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

            // Update online presence with world info
            {
                let mut online = state.online_players.write().await;
                if let Some(presence) = online.get_mut(&p.id) {
                    presence.world_id = Some(world_id);
                    presence.world_name = Some(world.name.clone());
                }
            }

            // Record recent players (other players already in this world)
            #[cfg(feature = "postgres")]
            if let Some(db) = state.db.as_ref() {
                let players_in_world = world.players.read().await;
                for (&other_player_id, _) in players_in_world.iter() {
                    if other_player_id != p.id {
                        let _ = db.add_recent_player(p.id, other_player_id, world_id).await;
                        let _ = db.add_recent_player(other_player_id, p.id, world_id).await;
                    }
                }
                // Record world history
                let _ = db.upsert_world_history(p.id, world_id, &world.name).await;
                // Record player session
                if !p.is_spectator {
                    let _ = db.upsert_player_session(p.id, world_id, corp_id as i64, true).await;
                }
            }

            let tick = world.world.lock().await.current_tick();

            if p.is_spectator {
                info!(
                    "Spectator {} joined world {}",
                    p.username, world_id
                );
            } else {
                info!(
                    "Player {} joined world {} as corp {}",
                    p.username, world_id, corp_id
                );
            }

            // Send proxy summary first if reconnecting from AI proxy
            if proxy_was_active {
                // Gather actual corp state for a meaningful summary
                let actions = {
                    let w = world.world.lock().await;
                    let mut summary_actions = Vec::new();

                    summary_actions.push(gt_common::protocol::ProxyAction {
                        tick: proxy_ticks,
                        description: format!("{} ticks elapsed while you were away", proxy_ticks),
                    });

                    if let Some(fin) = w.financials.get(&corp_id) {
                        summary_actions.push(gt_common::protocol::ProxyAction {
                            tick: proxy_ticks,
                            description: format!(
                                "Current cash: ${:.0}, revenue: ${:.0}/tick, costs: ${:.0}/tick",
                                fin.cash, fin.revenue_per_tick, fin.cost_per_tick
                            ),
                        });
                        if fin.debt > 0 {
                            summary_actions.push(gt_common::protocol::ProxyAction {
                                tick: proxy_ticks,
                                description: format!("Outstanding debt: ${:.0}", fin.debt),
                            });
                        }
                    }

                    if let Some(node_ids) = w.corp_infra_nodes.get(&corp_id) {
                        let total = node_ids.len();
                        let damaged = node_ids.iter()
                            .filter(|nid| w.healths.get(nid).map(|h| h.condition < 0.9).unwrap_or(false))
                            .count();
                        let mut desc = format!("Infrastructure: {} nodes", total);
                        if damaged > 0 {
                            desc.push_str(&format!(", {} damaged", damaged));
                        }
                        summary_actions.push(gt_common::protocol::ProxyAction {
                            tick: proxy_ticks,
                            description: desc,
                        });
                    }

                    summary_actions
                };

                let _ = forward_tx
                    .send(ServerMessage::ProxySummary {
                        ticks_elapsed: proxy_ticks,
                        actions,
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
                        if !p.is_spectator {
                            world.remove_player(&p.id).await;
                        }
                        let _ = world.broadcast_tx.send(ServerMessage::PlayerStatus {
                            player_id: p.id,
                            username: p.username.clone(),
                            status: PlayerConnectionStatus::Disconnected,
                        });
                    }
                    // Update online presence to remove world info
                    let mut online = state.online_players.write().await;
                    if let Some(presence) = online.get_mut(&p.id) {
                        presence.world_id = None;
                        presence.world_name = None;
                    }
                }
                p.corp_id = None;
            }
            Some(ServerMessage::CommandAck {
                success: true,
                error: None,
                seq: None,
                entity_id: None,
                effective_tick: None,
            })
        }

        ClientMessage::GameCommand { world_id, command, seq } => {
            let p = match player {
                Some(p) => p,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::NotAuthenticated,
                        message: "Not authenticated".to_string(),
                    });
                }
            };

            // Spectators cannot send game commands (double-check in case they
            // bypassed the outer check somehow)
            if p.is_spectator {
                return Some(ServerMessage::Error {
                    code: ErrorCode::PermissionDenied,
                    message: "Spectators cannot send game commands".to_string(),
                });
            }

            if p.world_id.as_ref() != Some(&world_id) {
                return Some(ServerMessage::Error {
                    code: ErrorCode::NotInWorld,
                    message: "Not in this world".to_string(),
                });
            }

            // Validate command parameters
            if let Err(reason) = super::validation::validate_command(&command) {
                return Some(ServerMessage::Error {
                    code: ErrorCode::InvalidCommand,
                    message: reason.to_string(),
                });
            }

            // Anti-cheat: verify the player owns the corporation targeted by the command
            if let Some(target_corp) = super::validation::command_target_corp(&command) {
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

            // Speed vote system: SetSpeed/TogglePause go through voting
            if matches!(command, Command::SetSpeed(_) | Command::TogglePause) {
                let requested_speed = match &command {
                    Command::SetSpeed(speed) => *speed,
                    Command::TogglePause => {
                        let w = world.world.lock().await;
                        if w.speed() == gt_common::types::GameSpeed::Paused {
                            gt_common::types::GameSpeed::Normal
                        } else {
                            gt_common::types::GameSpeed::Paused
                        }
                    }
                    _ => unreachable!(),
                };

                let is_creator = {
                    let creator = world.creator_id.read().await;
                    *creator == Some(p.id)
                };

                // Creator has override power -- set speed directly
                if is_creator {
                    let mut w = world.world.lock().await;
                    w.process_command(Command::SetSpeed(requested_speed));
                    let tick = w.current_tick();
                    drop(w);

                    // Clear any pending votes and broadcast
                    world.speed_votes.write().await.clear();
                    let _ = world.broadcast_tx.send(ServerMessage::SpeedVoteUpdate {
                        votes: vec![],
                        resolved_speed: requested_speed,
                    });

                    return Some(ServerMessage::CommandAck {
                        success: true,
                        error: None,
                        seq,
                        entity_id: None,
                        effective_tick: Some(tick),
                    });
                }

                // Non-creator: register vote
                let speed_str = format!("{:?}", requested_speed);
                world.speed_votes.write().await.insert(p.id, speed_str);

                // Tally votes and resolve by majority
                let players = world.players.read().await;
                let total_players = players.len();
                let votes = world.speed_votes.read().await;
                let mut vote_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
                for speed in votes.values() {
                    *vote_counts.entry(speed.clone()).or_insert(0) += 1;
                }
                drop(players);

                // Find the speed with the most votes
                let majority_threshold = (total_players / 2) + 1;
                let resolved = vote_counts.iter()
                    .filter(|(_, count)| **count >= majority_threshold)
                    .max_by_key(|(_, count)| *count)
                    .map(|(speed, _)| speed.clone());

                if let Some(speed_str) = resolved {
                    let resolved_speed = match speed_str.as_str() {
                        "Paused" => gt_common::types::GameSpeed::Paused,
                        "Normal" => gt_common::types::GameSpeed::Normal,
                        "Fast" => gt_common::types::GameSpeed::Fast,
                        "VeryFast" => gt_common::types::GameSpeed::VeryFast,
                        "Ultra" => gt_common::types::GameSpeed::Ultra,
                        _ => gt_common::types::GameSpeed::Normal,
                    };

                    let mut w = world.world.lock().await;
                    w.process_command(Command::SetSpeed(resolved_speed));
                    let tick = w.current_tick();
                    drop(w);

                    // Clear votes after resolution
                    world.speed_votes.write().await.clear();
                    let _ = world.broadcast_tx.send(ServerMessage::SpeedVoteUpdate {
                        votes: vec![],
                        resolved_speed,
                    });

                    return Some(ServerMessage::CommandAck {
                        success: true,
                        error: None,
                        seq,
                        entity_id: None,
                        effective_tick: Some(tick),
                    });
                }

                // No majority yet -- broadcast vote tally
                let vote_entries: Vec<gt_common::protocol::SpeedVoteEntry> = votes.iter().map(|(pid, speed)| {
                    // Look up username from state
                    gt_common::protocol::SpeedVoteEntry {
                        username: pid.to_string(), // Will be resolved below
                        speed: match speed.as_str() {
                            "Paused" => gt_common::types::GameSpeed::Paused,
                            "Normal" => gt_common::types::GameSpeed::Normal,
                            "Fast" => gt_common::types::GameSpeed::Fast,
                            "VeryFast" => gt_common::types::GameSpeed::VeryFast,
                            "Ultra" => gt_common::types::GameSpeed::Ultra,
                            _ => gt_common::types::GameSpeed::Normal,
                        },
                    }
                }).collect();
                drop(votes);

                let current_speed = {
                    let w = world.world.lock().await;
                    w.speed()
                };
                let _ = world.broadcast_tx.send(ServerMessage::SpeedVoteUpdate {
                    votes: vote_entries,
                    resolved_speed: current_speed,
                });

                return Some(ServerMessage::CommandAck {
                    success: true,
                    error: Some("Speed vote registered, waiting for majority".to_string()),
                    seq,
                    entity_id: None,
                    effective_tick: None,
                });
            }

            // Process the command using the player's corp and collect result
            let corp_id = p.corp_id.unwrap_or(0);
            let mut w = world.world.lock().await;
            let tick = w.current_tick();
            let command_debug = format!("{:?}", command);
            let result = w.process_command_for_corp(command, corp_id);
            drop(w);

            state.log_command(p.id, command_debug, tick).await;

            // Broadcast delta ops to all players if command produced visible changes
            if result.success && !result.ops.is_empty() {
                let _ = world.broadcast_tx.send(ServerMessage::CommandBroadcast {
                    tick,
                    corp_id,
                    ops: result.ops,
                });
            }

            Some(ServerMessage::CommandAck {
                success: result.success,
                error: result.error,
                seq,
                entity_id: result.entity_id,
                effective_tick: Some(tick),
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
            // Serialize the full ECS world state so clients can load it
            let state_json = match w.save_game() {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to serialize world snapshot: {e}");
                    return Some(ServerMessage::Error {
                        code: ErrorCode::InternalError,
                        message: "Failed to serialize world snapshot".to_string(),
                    });
                }
            };

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
            // Validate chat message length
            if message.len() > MAX_CHAT_LENGTH {
                return Some(ServerMessage::Error {
                    code: ErrorCode::InvalidCommand,
                    message: format!("Chat message too long (max {} chars)", MAX_CHAT_LENGTH),
                });
            }

            // Sanitize: strip control chars, trim whitespace
            let sanitized = match sanitize_chat(&message) {
                Some(m) => m,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::InvalidCommand,
                        message: "Chat message is empty".to_string(),
                    });
                }
            };

            let p = match player {
                Some(p) => p,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::NotAuthenticated,
                        message: "Not authenticated".to_string(),
                    });
                }
            };

            let world_id = match &p.world_id {
                Some(id) => *id,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::InvalidCommand,
                        message: "Not in a world".to_string(),
                    });
                }
            };

            let world = match state.get_world(&world_id).await {
                Some(w) => w,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::InvalidCommand,
                        message: "World not found".to_string(),
                    });
                }
            };

            let sender_name = if p.is_spectator {
                format!("[Spectator] {}", p.username)
            } else {
                p.username.clone()
            };
            let _ = world.broadcast_tx.send(ServerMessage::ChatBroadcast {
                sender: sender_name,
                message: sanitized,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
            None // Chat is broadcast to all, sender sees it via broadcast
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

            // Enforce save size limit
            if save_data.len() > MAX_SAVE_SIZE {
                return Some(ServerMessage::Error {
                    code: ErrorCode::InvalidCommand,
                    message: format!(
                        "Save data too large ({:.1} MB, max {} MB)",
                        save_data.len() as f64 / 1_000_000.0,
                        MAX_SAVE_SIZE / 1_000_000
                    ),
                });
            }

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
                            seq: None,
                            entity_id: None,
                            effective_tick: None,
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

            let world = match state.get_world(&world_id).await {
                Some(w) => w,
                None => {
                    return Some(ServerMessage::Error {
                        code: ErrorCode::WorldNotFound,
                        message: "World not found".to_string(),
                    });
                }
            };

            let invite_code = world.invite_code.clone().unwrap_or_default();
            let world_name = world.name.clone();
            let from_username = p.username.clone();

            // Check if the friend is online and send them the invite
            let online = state.online_players.read().await;
            if let Some(friend_presence) = online.get(&friend_id) {
                if let Some(friend_world_id) = friend_presence.world_id {
                    if let Some(friend_world) = state.worlds.read().await.get(&friend_world_id) {
                        let _ = friend_world.broadcast_tx.send(ServerMessage::WorldInvite {
                            from_username: from_username.clone(),
                            world_id,
                            world_name: world_name.clone(),
                            invite_code: invite_code.clone(),
                        });
                    }
                }
            }

            Some(ServerMessage::CommandAck {
                success: true,
                error: None,
                seq: None,
                entity_id: None,
                effective_tick: None,
            })
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
                world_id: existing_session
                    .as_ref()
                    .and_then(|s| s.world_id),
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
            register_presence(state, account.id, &account.username, &account.display_name, &account.avatar_id).await;

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
                    register_presence(state, account.id, &account.username, &account.display_name, &account.avatar_id).await;

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
                        world_id: existing_session
                            .as_ref()
                            .and_then(|s| s.world_id),
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
                    register_presence(state, player_id, &claims.username, &None, "tower_01").await;

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
