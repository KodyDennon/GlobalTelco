use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid::Uuid;

use gt_common::protocol::{
    deserialize_msgpack, serialize_msgpack, AuthRequest, AuthResponse, ClientMessage, CorpDelta,
    ErrorCode, PlayerConnectionStatus, ServerMessage,
};

use gt_common::commands::Command;
use gt_common::types::EntityId;

use crate::auth;
use crate::state::{AppState, ConnectedPlayer};

/// Extract the corporation EntityId that a command targets, if any.
/// Used for anti-cheat validation: ensures players can only issue commands
/// that affect their own corporation.
fn command_target_corp(command: &Command) -> Option<EntityId> {
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

/// Validate command parameters before forwarding to the simulation.
/// Returns an error message if validation fails.
fn validate_command(command: &Command) -> Result<(), &'static str> {
    match command {
        Command::BuildNode { lon, lat, .. } => {
            // Spatial validation: coordinates must be finite and within world bounds
            if !lon.is_finite() || !lat.is_finite() {
                return Err("Coordinates must be finite numbers");
            }
            if *lon < -180.0 || *lon > 180.0 {
                return Err("Longitude must be between -180 and 180");
            }
            if *lat < -90.0 || *lat > 90.0 {
                return Err("Latitude must be between -90 and 90");
            }
        }
        Command::TakeLoan { amount, .. } => {
            if *amount <= 0 {
                return Err("Loan amount must be positive");
            }
        }
        Command::HireEmployee { role, .. } => {
            if role.trim().is_empty() {
                return Err("Employee role cannot be empty");
            }
        }
        Command::ProposeContract { terms, .. } => {
            if terms.len() > 10_000 {
                return Err("Contract terms too long (max 10,000 chars)");
            }
        }
        Command::RepayLoan { amount, .. } => {
            if *amount <= 0 {
                return Err("Repayment amount must be positive");
            }
        }
        Command::PlaceBid { amount, .. } => {
            if *amount <= 0 {
                return Err("Bid amount must be positive");
            }
        }
        Command::ProposeAcquisition { offer, .. } => {
            if *offer <= 0 {
                return Err("Acquisition offer must be positive");
            }
        }
        Command::StartLobbying { budget, .. } => {
            if *budget <= 0 {
                return Err("Lobbying budget must be positive");
            }
        }
        Command::ProposeCoOwnership { share_pct, .. } => {
            if *share_pct <= 0.0 || *share_pct > 100.0 {
                return Err("Co-ownership share must be between 0 and 100");
            }
        }
        Command::ProposeBuyout { price, .. } => {
            if *price <= 0 {
                return Err("Buyout price must be positive");
            }
        }
        _ => {}
    }
    Ok(())
}

/// Per-type sliding window rate limiter.
/// Different command categories have different rate limits.
struct RateLimiter {
    build_timestamps: Vec<std::time::Instant>,
    financial_timestamps: Vec<std::time::Instant>,
    research_timestamps: Vec<std::time::Instant>,
    espionage_timestamps: Vec<std::time::Instant>,
    general_timestamps: Vec<std::time::Instant>,
    chat_timestamps: Vec<std::time::Instant>,
}

/// Command category for rate limiting
enum CommandCategory {
    Build,      // BuildNode, BuildEdge, UpgradeNode, DecommissionNode
    Financial,  // TakeLoan, RepayLoan, SetBudget, PurchaseInsurance, etc.
    Research,   // StartResearch, CancelResearch
    Espionage,  // LaunchEspionage, LaunchSabotage
    General,    // Everything else
}

fn categorize_command(command: &Command) -> CommandCategory {
    match command {
        Command::BuildNode { .. }
        | Command::BuildEdge { .. }
        | Command::UpgradeNode { .. }
        | Command::DecommissionNode { .. }
        | Command::RepairNode { .. }
        | Command::EmergencyRepair { .. } => CommandCategory::Build,

        Command::TakeLoan { .. }
        | Command::RepayLoan { .. }
        | Command::SetBudget { .. }
        | Command::PurchaseInsurance { .. }
        | Command::CancelInsurance { .. }
        | Command::PlaceBid { .. }
        | Command::ProposeAcquisition { .. }
        | Command::ProposeContract { .. } => CommandCategory::Financial,

        Command::StartResearch { .. }
        | Command::CancelResearch { .. } => CommandCategory::Research,

        Command::LaunchEspionage { .. }
        | Command::LaunchSabotage { .. } => CommandCategory::Espionage,

        _ => CommandCategory::General,
    }
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            build_timestamps: Vec::new(),
            financial_timestamps: Vec::new(),
            research_timestamps: Vec::new(),
            espionage_timestamps: Vec::new(),
            general_timestamps: Vec::new(),
            chat_timestamps: Vec::new(),
        }
    }

    fn check_command(&mut self) -> bool {
        // Global fallback: 10 commands/sec across all types
        let now = std::time::Instant::now();
        let cutoff = now - std::time::Duration::from_secs(1);
        self.general_timestamps.retain(|t| *t > cutoff);
        if self.general_timestamps.len() >= 10 {
            return false;
        }
        self.general_timestamps.push(now);
        true
    }

    /// Per-type rate limit check. Returns true if allowed.
    fn check_typed_command(&mut self, command: &Command) -> bool {
        let now = std::time::Instant::now();
        let category = categorize_command(command);

        let (timestamps, window, max) = match category {
            CommandCategory::Build => (&mut self.build_timestamps, std::time::Duration::from_secs(1), 3usize),
            CommandCategory::Financial => (&mut self.financial_timestamps, std::time::Duration::from_secs(1), 2),
            CommandCategory::Research => (&mut self.research_timestamps, std::time::Duration::from_secs(5), 1),
            CommandCategory::Espionage => (&mut self.espionage_timestamps, std::time::Duration::from_secs(30), 1),
            CommandCategory::General => (&mut self.general_timestamps, std::time::Duration::from_secs(1), 10),
        };

        let cutoff = now - window;
        timestamps.retain(|t| *t > cutoff);
        if timestamps.len() >= max {
            return false;
        }
        timestamps.push(now);
        true
    }

    fn check_chat(&mut self) -> bool {
        let now = std::time::Instant::now();
        let cutoff = now - std::time::Duration::from_secs(10);
        self.chat_timestamps.retain(|t| *t > cutoff);
        if self.chat_timestamps.len() >= 5 {
            return false;
        }
        self.chat_timestamps.push(now);
        true
    }
}

/// Maximum connections per IP address
const MAX_CONNECTIONS_PER_IP: usize = 10;
/// Time allowed for authentication after WebSocket upgrade
const AUTH_TIMEOUT_SECS: u64 = 10;
/// Maximum chat message length in bytes
const MAX_CHAT_LENGTH: usize = 500;
/// Maximum cloud save size in bytes (50 MB)
const MAX_SAVE_SIZE: usize = 50_000_000;

/// Sanitize a chat message: strip control characters, trim whitespace.
/// Returns None if the message is empty after sanitization.
fn sanitize_chat(message: &str) -> Option<String> {
    let cleaned: String = message
        .chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Round a money value to an approximate range for intel level 1 (basic financials).
/// Returns the value rounded to the nearest "bucket" so the player gets a rough idea
/// but not exact numbers. Uses significant-digit rounding:
/// - Values < 10,000 → round to nearest 1,000
/// - Values < 1,000,000 → round to nearest 100,000
/// - Values >= 1,000,000 → round to nearest 1,000,000
fn approximate_money(value: i64) -> i64 {
    let abs = value.unsigned_abs();
    let bucket = if abs < 10_000 {
        1_000
    } else if abs < 1_000_000 {
        100_000
    } else {
        1_000_000
    };
    let rounded = ((abs + bucket / 2) / bucket) * bucket;
    if value >= 0 {
        rounded as i64
    } else {
        -(rounded as i64)
    }
}

/// Filter a TickUpdate for per-player data visibility using graduated intel levels.
///
/// Intel levels (per spy_corp → target_corp pair):
///   0 = Infrastructure positions only (node_count visible, financials/ops hidden)
///   1 = Basic financials (revenue, cost, cash, debt as approximate ranges)
///   2 = Detailed financials (exact revenue, cost, cash, debt numbers)
///   3 = Full operational data (utilization, health, throughput) + exact financials
///
/// Rules:
/// - Spectators see ALL data (no filtering).
/// - A player's OWN corp data is always fully visible.
/// - Competitor data is filtered based on the player's intel level against that competitor.
/// - Intel levels are populated by the espionage system (covert_ops).
fn filter_tick_update_for_player(
    update: &ServerMessage,
    player_corp_id: Option<EntityId>,
    is_spectator: bool,
    intel_levels: &std::collections::HashMap<EntityId, u8>,
) -> ServerMessage {
    // Spectators see everything
    if is_spectator {
        return update.clone();
    }

    match update {
        ServerMessage::TickUpdate {
            tick,
            corp_updates,
            events,
        } => {
            let filtered_updates: Vec<CorpDelta> = corp_updates
                .iter()
                .map(|delta| {
                    // Player's own corp data is always fully visible
                    if Some(delta.corp_id) == player_corp_id {
                        return delta.clone();
                    }

                    // Look up intel level for this competitor
                    let intel = intel_levels.get(&delta.corp_id).copied().unwrap_or(0);

                    match intel {
                        0 => {
                            // Level 0: infrastructure positions only (node_count visible)
                            CorpDelta {
                                corp_id: delta.corp_id,
                                node_count: delta.node_count,
                                cash: None,
                                revenue: None,
                                cost: None,
                                debt: None,
                                avg_utilization: None,
                                avg_health: None,
                                total_throughput: None,
                            }
                        }
                        1 => {
                            // Level 1: basic financials (approximate ranges)
                            CorpDelta {
                                corp_id: delta.corp_id,
                                node_count: delta.node_count,
                                cash: delta.cash.map(approximate_money),
                                revenue: delta.revenue.map(approximate_money),
                                cost: delta.cost.map(approximate_money),
                                debt: delta.debt.map(approximate_money),
                                avg_utilization: None,
                                avg_health: None,
                                total_throughput: None,
                            }
                        }
                        2 => {
                            // Level 2: exact financials, no operational data
                            CorpDelta {
                                corp_id: delta.corp_id,
                                node_count: delta.node_count,
                                cash: delta.cash,
                                revenue: delta.revenue,
                                cost: delta.cost,
                                debt: delta.debt,
                                avg_utilization: None,
                                avg_health: None,
                                total_throughput: None,
                            }
                        }
                        _ => {
                            // Level 3+: full data (exact financials + operational)
                            delta.clone()
                        }
                    }
                })
                .collect();

            // Filter events: global events (empty related_corps) go to everyone;
            // private events only go to the relevant corporations.
            let filtered_events: Vec<gt_common::events::GameEvent> = events
                .iter()
                .filter(|event| {
                    let corps = event.related_corps();
                    // Empty = global event, send to all
                    if corps.is_empty() {
                        return true;
                    }
                    // Send to player if their corp is in the related list
                    if let Some(pc) = player_corp_id {
                        corps.contains(&pc)
                    } else {
                        false
                    }
                })
                .cloned()
                .collect();

            ServerMessage::TickUpdate {
                tick: *tick,
                corp_updates: filtered_updates,
                events: filtered_events,
            }
        }
        // Non-TickUpdate messages pass through unmodified
        other => other.clone(),
    }
}

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

            // Check if player is banned from this world
            if world.banned_players.read().await.contains(&p.id) {
                return Some(ServerMessage::Error {
                    code: ErrorCode::PermissionDenied,
                    message: "You are banned from this world".to_string(),
                });
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
                    // New player -- create corporation
                    let mut w = world.world.lock().await;
                    let new_corp = w.allocate_entity();
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
            if let Err(reason) = validate_command(&command) {
                return Some(ServerMessage::Error {
                    code: ErrorCode::InvalidCommand,
                    message: reason.to_string(),
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

                // Creator has override power — set speed directly
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

                // No majority yet — broadcast vote tally
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

            if let Some(p) = player {
                if let Some(world_id) = &p.world_id {
                    if let Some(world) = state.get_world(world_id).await {
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
    }
}

async fn handle_auth(
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

            info!("Guest player joined: {}", account.username);

            ServerMessage::AuthResult(AuthResponse::GuestSuccess {
                player_id: account.id,
                username: account.username,
            })
        }
    }
}
