use std::sync::Arc;

use tokio::sync::broadcast;
use tracing::{error, info, warn};

use gt_common::protocol::{ErrorCode, PlayerConnectionStatus, ServerMessage};

use crate::state::{AppState, ConnectedPlayer};

use super::filtering::filter_tick_update_for_player;
use super::MAX_CHAT_LENGTH;
use super::chat::sanitize_chat;
use super::MAX_SAVE_SIZE;

/// Handle a JoinWorld request: create/reconnect corporation, subscribe to broadcasts,
/// notify other players, and return WorldJoined response.
pub(crate) async fn handle_join_world(
    world_id: uuid::Uuid,
    state: &Arc<AppState>,
    player: &mut ConnectedPlayer,
    world_broadcast_rx: &mut Option<broadcast::Receiver<ServerMessage>>,
    forward_tx: &tokio::sync::mpsc::Sender<ServerMessage>,
) -> ServerMessage {
    let world = match state.get_world(&world_id).await {
        Some(w) => w,
        None => {
            return ServerMessage::Error {
                code: ErrorCode::WorldNotFound,
                message: "World not found".to_string(),
            };
        }
    };

    // Check if player is banned from this world (in-memory first, then DB)
    if world.banned_players.read().await.contains(&player.id) {
        return ServerMessage::Error {
            code: ErrorCode::PermissionDenied,
            message: "You are banned from this world".to_string(),
        };
    }
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        if let Ok(true) = db.is_banned(player.id, Some(world_id)).await {
            // Also add to in-memory set for future fast checks
            world.banned_players.write().await.insert(player.id);
            return ServerMessage::Error {
                code: ErrorCode::PermissionDenied,
                message: "You are banned from this world".to_string(),
            };
        }
    }

    // Spectators don't count toward the player limit
    if !player.is_spectator && world.is_full().await {
        return ServerMessage::Error {
            code: ErrorCode::WorldFull,
            message: "World is full".to_string(),
        };
    }

    // Spectators don't get a corporation or participate in the world
    let (corp_id, proxy_ticks, proxy_was_active) = if player.is_spectator {
        // Spectator: no corp, no reconnection logic
        (0u64, 0u64, false)
    } else {
        // Check if this player already has a corp in this world (reconnection)
        let existing_players = world.players.read().await;
        if let Some(&existing_corp) = existing_players.get(&player.id) {
            // Reconnecting -- deactivate AI proxy and build summary
            let mut w = world.world.lock().await;
            let was_proxy = w
                .ai_states
                .get(&existing_corp)
                .map(|ai| ai.proxy_mode)
                .unwrap_or(false);
            let ticks = w.current_tick();
            // Remove AI proxy
            w.ai_states.shift_remove(&existing_corp);

            #[cfg(feature = "postgres")]
            if let Some(db) = state.db.as_ref() {
                let _ = db.set_player_connected(player.id, world_id).await;
            }

            info!(
                "Player {} reconnected to world {} (corp {}, proxy_was_active: {})",
                player.username, world_id, existing_corp, was_proxy
            );

            (existing_corp, ticks, was_proxy)
        } else {
            drop(existing_players);
            // New player -- create corporation with full components
            let mut w = world.world.lock().await;
            let new_corp = w.create_player_corporation(&player.username);
            (new_corp, 0, false)
        }
    };

    if !player.is_spectator {
        // Add player to world's player map
        world.add_player(player.id, corp_id).await;

        // First non-spectator player becomes the creator (has speed override)
        let mut creator = world.creator_id.write().await;
        if creator.is_none() {
            *creator = Some(player.id);
        }
        drop(creator);
    }

    player.world_id = Some(world_id);
    if !player.is_spectator {
        player.corp_id = Some(corp_id);
    }

    // Subscribe to world broadcasts with per-player filtering
    *world_broadcast_rx = Some(world.broadcast_tx.subscribe());

    if let Some(mut rx) = world_broadcast_rx.take() {
        let tx = forward_tx.clone();
        let player_corp_id = player.corp_id;
        let is_spectator = player.is_spectator;
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
        player_id: player.id,
        username: player.username.clone(),
        status: PlayerConnectionStatus::Connected,
    });

    // Update online presence with world info
    {
        let mut online = state.online_players.write().await;
        if let Some(presence) = online.get_mut(&player.id) {
            presence.world_id = Some(world_id);
            presence.world_name = Some(world.name.clone());
        }
    }

    // Record recent players (other players already in this world)
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        let players_in_world = world.players.read().await;
        for (&other_player_id, _) in players_in_world.iter() {
            if other_player_id != player.id {
                let _ = db
                    .add_recent_player(player.id, other_player_id, world_id)
                    .await;
                let _ = db
                    .add_recent_player(other_player_id, player.id, world_id)
                    .await;
            }
        }
        // Record world history
        let _ = db
            .upsert_world_history(player.id, world_id, &world.name)
            .await;
        // Record player session
        if !player.is_spectator {
            let _ = db
                .upsert_player_session(player.id, world_id, corp_id as i64, true)
                .await;
        }
    }

    let tick = world.world.lock().await.current_tick();

    if player.is_spectator {
        info!("Spectator {} joined world {}", player.username, world_id);
    } else {
        info!(
            "Player {} joined world {} as corp {}",
            player.username, world_id, corp_id
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
                let damaged = node_ids
                    .iter()
                    .filter(|nid| {
                        w.healths
                            .get(*nid)
                            .map(|h| h.condition < 0.9)
                            .unwrap_or(false)
                    })
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

    ServerMessage::WorldJoined {
        world_id,
        corp_id,
        tick,
    }
}

/// Handle a LeaveWorld request: remove player from world, notify others.
pub(crate) async fn handle_leave_world(
    state: &Arc<AppState>,
    player: &mut ConnectedPlayer,
) -> ServerMessage {
    if let Some(world_id) = player.world_id.take() {
        if let Some(world) = state.get_world(&world_id).await {
            if !player.is_spectator {
                world.remove_player(&player.id).await;
            }
            let _ = world.broadcast_tx.send(ServerMessage::PlayerStatus {
                player_id: player.id,
                username: player.username.clone(),
                status: PlayerConnectionStatus::Disconnected,
            });
        }
        // Update online presence to remove world info
        let mut online = state.online_players.write().await;
        if let Some(presence) = online.get_mut(&player.id) {
            presence.world_id = None;
            presence.world_name = None;
        }
    }
    player.corp_id = None;

    ServerMessage::CommandAck {
        success: true,
        error: None,
        seq: None,
        entity_id: None,
        effective_tick: None,
    }
}

/// Handle a RequestSnapshot: serialize and return the full ECS world state.
pub(crate) async fn handle_request_snapshot(
    world_id: uuid::Uuid,
    state: &Arc<AppState>,
) -> ServerMessage {
    let world = match state.get_world(&world_id).await {
        Some(w) => w,
        None => {
            return ServerMessage::Error {
                code: ErrorCode::WorldNotFound,
                message: "World not found".to_string(),
            };
        }
    };

    let w = world.world.lock().await;
    let tick = w.current_tick();
    // Serialize the full ECS world state so clients can load it
    let state_json = match w.save_game() {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to serialize world snapshot: {e}");
            return ServerMessage::Error {
                code: ErrorCode::InternalError,
                message: "Failed to serialize world snapshot".to_string(),
            };
        }
    };

    ServerMessage::Snapshot { tick, state_json }
}

/// Handle a Ping message: respond with Pong including server timestamp.
pub(crate) fn handle_ping(timestamp: u64) -> ServerMessage {
    let server_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    ServerMessage::Pong {
        timestamp,
        server_time,
    }
}

/// Handle a Chat message: validate, sanitize, broadcast, and persist.
pub(crate) async fn handle_chat(
    message: String,
    state: &Arc<AppState>,
    player: &ConnectedPlayer,
) -> Option<ServerMessage> {
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

    let world_id = match &player.world_id {
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

    let sender_name = if player.is_spectator {
        format!("[Spectator] {}", player.username)
    } else {
        player.username.clone()
    };
    let _ = world.broadcast_tx.send(ServerMessage::ChatBroadcast {
        sender: sender_name,
        message: sanitized.clone(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    });

    // Persist chat to DB
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        let db = db.clone();
        let wid = world_id;
        let aid = player.id;
        let uname = player.username.clone();
        let msg_text = sanitized;
        tokio::spawn(async move {
            let _ = db.insert_chat_message(wid, aid, &uname, &msg_text).await;
        });
    }

    None // Chat is broadcast to all, sender sees it via broadcast
}

/// Handle an UploadSave request: validate size and persist to database.
pub(crate) async fn handle_upload_save(
    slot: i32,
    name: String,
    save_data: Vec<u8>,
    tick: u64,
    config_json: String,
    state: &Arc<AppState>,
    player: &ConnectedPlayer,
) -> ServerMessage {
    // Enforce save size limit
    if save_data.len() > MAX_SAVE_SIZE {
        return ServerMessage::Error {
            code: ErrorCode::InvalidCommand,
            message: format!(
                "Save data too large ({:.1} MB, max {} MB)",
                save_data.len() as f64 / 1_000_000.0,
                MAX_SAVE_SIZE / 1_000_000
            ),
        };
    }

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        let config: serde_json::Value =
            serde_json::from_str(&config_json).unwrap_or(serde_json::json!({}));
        match db
            .save_cloud(player.id, slot, &name, &save_data, tick as i64, &config)
            .await
        {
            Ok(_) => {
                return ServerMessage::SaveUploaded {
                    slot,
                    success: true,
                };
            }
            Err(e) => {
                warn!("Cloud save upload failed: {e}");
                return ServerMessage::SaveUploaded {
                    slot,
                    success: false,
                };
            }
        }
    }

    ServerMessage::Error {
        code: ErrorCode::InternalError,
        message: "Database not available".to_string(),
    }
}

/// Handle a RequestSaves request: list cloud saves for the player.
pub(crate) async fn handle_request_saves(
    state: &Arc<AppState>,
    player: &ConnectedPlayer,
) -> ServerMessage {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.list_cloud_saves(player.id).await {
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
                return ServerMessage::SaveList { saves };
            }
            Err(e) => {
                warn!("Failed to list cloud saves: {e}");
            }
        }
    }

    ServerMessage::SaveList { saves: vec![] }
}

/// Handle a DownloadSave request: retrieve a cloud save by slot.
pub(crate) async fn handle_download_save(
    slot: i32,
    state: &Arc<AppState>,
    player: &ConnectedPlayer,
) -> ServerMessage {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.load_cloud_save(player.id, slot).await {
            Ok(Some(data)) => {
                return ServerMessage::SaveData {
                    slot,
                    save_data: data,
                };
            }
            Ok(None) => {
                return ServerMessage::Error {
                    code: ErrorCode::InvalidCommand,
                    message: "Save not found".to_string(),
                };
            }
            Err(e) => {
                warn!("Failed to load cloud save: {e}");
            }
        }
    }

    ServerMessage::Error {
        code: ErrorCode::InternalError,
        message: "Database not available".to_string(),
    }
}

/// Handle a DeleteSave request: remove a cloud save by slot.
pub(crate) async fn handle_delete_save(
    slot: i32,
    state: &Arc<AppState>,
    player: &ConnectedPlayer,
) -> ServerMessage {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.delete_cloud_save(player.id, slot).await {
            Ok(deleted) => {
                return ServerMessage::CommandAck {
                    success: deleted,
                    error: if deleted {
                        None
                    } else {
                        Some("Save not found".to_string())
                    },
                    seq: None,
                    entity_id: None,
                    effective_tick: None,
                };
            }
            Err(e) => {
                warn!("Failed to delete cloud save: {e}");
            }
        }
    }

    ServerMessage::Error {
        code: ErrorCode::InternalError,
        message: "Database not available".to_string(),
    }
}

/// Handle an InviteFriend request: send a world invite to an online friend.
pub(crate) async fn handle_invite_friend(
    friend_id: uuid::Uuid,
    world_id: uuid::Uuid,
    state: &Arc<AppState>,
    player: &ConnectedPlayer,
) -> ServerMessage {
    let world = match state.get_world(&world_id).await {
        Some(w) => w,
        None => {
            return ServerMessage::Error {
                code: ErrorCode::WorldNotFound,
                message: "World not found".to_string(),
            };
        }
    };

    let invite_code = world.invite_code.clone().unwrap_or_default();
    let world_name = world.name.clone();
    let from_username = player.username.clone();

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

    ServerMessage::CommandAck {
        success: true,
        error: None,
        seq: None,
        entity_id: None,
        effective_tick: None,
    }
}
