mod chat;
mod filtering;
mod handler;
mod rate_limit;
mod validation;

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use gt_common::protocol::{
    deserialize_msgpack, serialize_msgpack, AuthResponse, ClientMessage, ErrorCode,
    PlayerConnectionStatus, ServerMessage,
};

use crate::state::{AppState, ConnectedPlayer};

use handler::{handle_auth, handle_client_message, unregister_presence};
use rate_limit::RateLimiter;

/// Maximum connections per IP address
const MAX_CONNECTIONS_PER_IP: usize = 10;
/// Time allowed for authentication after WebSocket upgrade
const AUTH_TIMEOUT_SECS: u64 = 30;
/// Maximum chat message length in bytes
pub(crate) const MAX_CHAT_LENGTH: usize = 500;
/// Maximum cloud save size in bytes (50 MB)
pub(crate) const MAX_SAVE_SIZE: usize = 50_000_000;

/// Handle an individual WebSocket connection
pub async fn handle_socket(socket: WebSocket, state: Arc<AppState>, ip: IpAddr) {
    // Per-IP connection limit
    let conn_count = state.ip_connect(ip).await;
    if conn_count > MAX_CONNECTIONS_PER_IP {
        state.ip_disconnect(ip).await;
        warn!("Rejected WebSocket from {ip}: too many connections ({conn_count})");
        return;
    }

    let (mut sender, mut receiver) = socket.split();

    let mut player: Option<ConnectedPlayer> = None;
    let mut world_broadcast_rx: Option<broadcast::Receiver<ServerMessage>> = None;
    let mut rate_limiter = RateLimiter::new();
    // Sequence number dedup: track the highest seq seen from this client
    let mut last_seq: u64 = 0;

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

    // Auth timeout: first message must be an Auth message within AUTH_TIMEOUT_SECS
    let auth_result = tokio::time::timeout(Duration::from_secs(AUTH_TIMEOUT_SECS), async {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    match deserialize_msgpack::<ClientMessage>(&data) {
                        Ok(ClientMessage::Auth(auth_req)) => return Some(auth_req),
                        Ok(_) => {
                            // Non-auth message before auth -- reject
                            return None;
                        }
                        Err(e) => {
                            warn!("Failed to deserialize auth message: {e}");
                            continue;
                        }
                    }
                }
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(ClientMessage::Auth(auth_req)) => return Some(auth_req),
                        Ok(_) => {
                            // Non-auth message before auth -- reject
                            return None;
                        }
                        Err(e) => {
                            warn!("Failed to deserialize JSON auth message: {e}");
                            continue;
                        }
                    }
                }
                Ok(Message::Close(_)) => return None,
                Err(_) => return None,
                _ => continue,
            }
        }
        None
    })
    .await;

    match auth_result {
        Ok(Some(auth_req)) => {
            let response = handle_auth(auth_req, &state, &mut player).await;

            // Check if auth failed before sending response
            let auth_failed = matches!(
                &response,
                ServerMessage::AuthResult(AuthResponse::Failed { .. })
            );

            if forward_tx.send(response).await.is_err() {
                state.ip_disconnect(ip).await;
                let _ = forward_sender.await;
                return;
            }

            if auth_failed || player.is_none() {
                // Give the client a moment to receive the error before closing
                tokio::time::sleep(Duration::from_millis(100)).await;
                state.ip_disconnect(ip).await;
                let _ = forward_sender.await;
                return;
            }
        }
        Ok(None) => {
            // Client sent a non-auth message first, or closed before auth
            let _ = forward_tx
                .send(ServerMessage::Error {
                    code: ErrorCode::NotAuthenticated,
                    message: "First message must be an Auth message".to_string(),
                })
                .await;
            tokio::time::sleep(Duration::from_millis(100)).await;
            state.ip_disconnect(ip).await;
            let _ = forward_sender.await;
            return;
        }
        Err(_) => {
            // Timeout
            warn!("WebSocket from {ip}: auth timeout (no auth message within {AUTH_TIMEOUT_SECS}s)");
            let _ = forward_tx
                .send(ServerMessage::Error {
                    code: ErrorCode::NotAuthenticated,
                    message: format!(
                        "Authentication timeout: must send Auth message within {AUTH_TIMEOUT_SECS} seconds"
                    ),
                })
                .await;
            tokio::time::sleep(Duration::from_millis(100)).await;
            state.ip_disconnect(ip).await;
            let _ = forward_sender.await;
            return;
        }
    }

    // Main receive loop (player is authenticated at this point)
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

        // Increment WS message counter
        state.ws_message_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Spectators cannot send game commands, uploads, or deletes
        if let Some(ref p) = player {
            if p.is_spectator {
                match &msg {
                    ClientMessage::GameCommand { .. }
                    | ClientMessage::UploadSave { .. }
                    | ClientMessage::DeleteSave { .. } => {
                        let _ = forward_tx
                            .send(ServerMessage::Error {
                                code: ErrorCode::PermissionDenied,
                                message: "Spectators cannot send game commands".to_string(),
                            })
                            .await;
                        continue;
                    }
                    // Spectators can still: Auth, JoinWorld, LeaveWorld, RequestSnapshot,
                    // Ping, Chat, RequestSaves, DownloadSave
                    _ => {}
                }
            }
        }

        // Rate limit commands and chat (per-type for game commands)
        let is_rate_limited = match &msg {
            ClientMessage::GameCommand { command, .. } => !rate_limiter.check_typed_command(command),
            ClientMessage::UploadSave { .. }
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

        // Sequence number dedup: reject commands with already-seen seq numbers
        if let ClientMessage::GameCommand { seq: Some(seq_val), .. } = &msg {
            if *seq_val <= last_seq {
                let _ = forward_tx.send(ServerMessage::CommandAck {
                    success: false,
                    error: Some("Duplicate command (seq already processed)".to_string()),
                    seq: Some(*seq_val),
                    entity_id: None,
                    effective_tick: None,
                }).await;
                continue;
            }
            last_seq = *seq_val;
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

    // Cleanup: decrement IP connection count and activate AI proxy
    state.ip_disconnect(ip).await;

    if let Some(p) = &player {
        info!(
            "{} {} disconnected",
            if p.is_spectator {
                "Spectator"
            } else {
                "Player"
            },
            p.username
        );

        // Spectators don't have corporations and don't need AI proxy
        if !p.is_spectator {
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
                            bankruptcy_ticks: 0,
                        };
                        w.ai_states.insert(corp_id, proxy_ai);
                        info!(
                            "AI proxy activated for corp {} (player {})",
                            corp_id, p.username
                        );
                    }

                    // Don't remove the player from the world -- they're just disconnected
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
        }

        // Remove online presence and notify friends
        unregister_presence(&state, p.id, &p.username).await;

        state.players.write().await.remove(&p.id);
    }

    // Wait for forward task to finish
    let _ = forward_sender.await;
}
