use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use gt_common::types::{GameSpeed, WorldConfig};

use crate::auth;
use crate::state::AppState;
use crate::tick;

use super::extract_admin_claims;
use super::worlds::default_max_players;

// ── Admin Endpoints ───────────────────────────────────────────────────────

pub(crate) async fn admin_list_players(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let players = state.players.read().await;
    let list: Vec<serde_json::Value> = players
        .values()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "username": p.username,
                "is_guest": p.is_guest,
                "is_admin": p.is_admin,
                "world_id": p.world_id,
                "corp_id": p.corp_id,
            })
        })
        .collect();

    (StatusCode::OK, Json(serde_json::json!({ "players": list })))
}

#[derive(Deserialize)]
pub(crate) struct KickRequest {
    player_id: Uuid,
}

pub(crate) async fn admin_kick_player(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<KickRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let kicked = state.kick_player(&body.player_id).await;
    if kicked {
        #[cfg(feature = "postgres")]
        if let Some(db) = state.db.as_ref() {
            let _ = db
                .insert_audit_log("admin", "kick_player", Some(&body.player_id.to_string()), None, None)
                .await;
        }
        (
            StatusCode::OK,
            Json(serde_json::json!({ "kicked": true, "player_id": body.player_id })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Player not found" })),
        )
    }
}

#[derive(Deserialize)]
pub(crate) struct PauseRequest {
    world_id: Uuid,
}

pub(crate) async fn admin_pause_world(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<PauseRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    match state.get_world(&body.world_id).await {
        Some(world) => {
            let mut w = world.world.lock().await;
            w.process_command(gt_common::commands::Command::TogglePause);
            let speed = w.speed();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "world_id": body.world_id,
                    "paused": speed == gt_common::types::GameSpeed::Paused,
                    "speed": format!("{:?}", speed),
                })),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "World not found" })),
        ),
    }
}

pub(crate) async fn admin_audit_log(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    // Use DB-backed paginated audit log when available
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.query_audit_log(100, 0, None).await {
            Ok((entries, total)) => {
                let result: Vec<serde_json::Value> = entries
                    .into_iter()
                    .map(|e| {
                        serde_json::json!({
                            "id": e.id,
                            "actor": e.actor,
                            "action": e.action,
                            "target": e.target,
                            "details": e.details,
                            "ip_address": e.ip_address,
                            "created_at": e.created_at.to_rfc3339(),
                        })
                    })
                    .collect();
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({ "audit_log": result, "total": total })),
                );
            }
            Err(_) => {
                // Fall through to in-memory log
            }
        }
    }

    // Fallback to in-memory audit log
    let log = state.get_audit_log().await;
    (StatusCode::OK, Json(serde_json::json!({ "audit_log": log })))
}

pub(crate) async fn admin_health(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let worlds = state.worlds.read().await;
    let players = state.players.read().await;
    let accounts = state.accounts.read().await;
    let audit_log = state.audit_log.read().await;
    let uptime = state.uptime_secs();

    // Per-world details
    let mut world_details = Vec::new();
    for instance in worlds.values() {
        let w = instance.world.lock().await;
        world_details.push(serde_json::json!({
            "id": instance.id,
            "name": instance.name,
            "tick": w.current_tick(),
            "speed": format!("{:?}", w.speed()),
            "player_count": instance.player_count().await,
            "max_players": instance.max_players,
            "tick_rate_ms": instance.tick_rate_ms,
            "era": format!("{:?}", w.config().starting_era),
            "map_size": format!("{:?}", w.config().map_size),
        }));
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_secs": uptime,
            "active_worlds": worlds.len(),
            "connected_players": players.len(),
            "registered_accounts": accounts.len(),
            "audit_log_entries": audit_log.len(),
            "worlds": world_details,
            "has_database": state.db.is_some(),
        })),
    )
}

#[derive(Deserialize)]
pub(crate) struct AdminCreateWorldRequest {
    name: String,
    #[serde(default)]
    config: Option<WorldConfig>,
    #[serde(default = "default_max_players")]
    max_players: u32,
}

pub(crate) async fn admin_create_world(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<AdminCreateWorldRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let config = body.config.unwrap_or_default();
    let world_id = state
        .create_world(body.name.clone(), config, body.max_players)
        .await;

    if let Some(world) = state.get_world(&world_id).await {
        #[cfg(feature = "postgres")]
        tick::spawn_world_tick_loop(
            world,
            state.db.clone(),
            #[cfg(feature = "r2")]
            state.r2.clone(),
        );
        #[cfg(not(feature = "postgres"))]
        tick::spawn_world_tick_loop(world);
    }

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "world_id": world_id,
            "name": body.name,
        })),
    )
}

pub(crate) async fn admin_delete_world(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    // Kick all players from this world first
    let mut kicked_players = Vec::new();
    {
        let mut players = state.players.write().await;
        players.retain(|id, p| {
            if p.world_id == Some(world_id) {
                kicked_players.push(*id);
                false
            } else {
                true
            }
        });
    }

    let removed = state.remove_world(&world_id).await;
    if removed {
        #[cfg(feature = "postgres")]
        if let Some(db) = state.db.as_ref() {
            let _ = db
                .insert_audit_log(
                    "admin",
                    "delete_world",
                    Some(&world_id.to_string()),
                    Some(&serde_json::json!({ "kicked_players": kicked_players.len() })),
                    None,
                )
                .await;
        }
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "deleted": true,
                "world_id": world_id,
                "kicked_players": kicked_players.len(),
            })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "World not found" })),
        )
    }
}

#[derive(Deserialize)]
pub(crate) struct SetSpeedRequest {
    speed: String,
}

pub(crate) async fn admin_set_speed(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<Uuid>,
    Json(body): Json<SetSpeedRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let target_speed = match body.speed.to_lowercase().as_str() {
        "paused" => GameSpeed::Paused,
        "normal" => GameSpeed::Normal,
        "fast" => GameSpeed::Fast,
        "veryfast" => GameSpeed::VeryFast,
        "ultra" => GameSpeed::Ultra,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid speed. Use: Paused, Normal, Fast, VeryFast, Ultra" })),
            );
        }
    };

    match state.get_world(&world_id).await {
        Some(world) => {
            let mut w = world.world.lock().await;
            w.process_command(gt_common::commands::Command::SetSpeed(target_speed));
            let speed = w.speed();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "world_id": world_id,
                    "speed": format!("{:?}", speed),
                    "paused": speed == GameSpeed::Paused,
                })),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "World not found" })),
        ),
    }
}

#[derive(Deserialize)]
pub(crate) struct BroadcastRequest {
    message: String,
    #[serde(default)]
    world_id: Option<Uuid>,
}

pub(crate) async fn admin_broadcast(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<BroadcastRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let msg = gt_common::protocol::ServerMessage::ChatBroadcast {
        sender: "ADMIN".to_string(),
        message: body.message.clone(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    if let Some(wid) = body.world_id {
        let sent = state.broadcast_to_world(&wid, msg).await;
        if sent {
            (
                StatusCode::OK,
                Json(serde_json::json!({ "broadcast": true, "scope": "world", "world_id": wid })),
            )
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "World not found" })),
            )
        }
    } else {
        // Broadcast to all worlds
        let worlds = state.worlds.read().await;
        for instance in worlds.values() {
            let _ = instance.broadcast_tx.send(msg.clone());
        }
        (
            StatusCode::OK,
            Json(serde_json::json!({ "broadcast": true, "scope": "all", "world_count": worlds.len() })),
        )
    }
}

// ── Ban / Unban Endpoints ──────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct BanRequest {
    world_id: Uuid,
    player_id: Uuid,
}

pub(crate) async fn admin_ban_player(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<BanRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let world = match state.get_world(&body.world_id).await {
        Some(w) => w,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "World not found" })),
            );
        }
    };

    // Add to in-memory ban list
    world.banned_players.write().await.insert(body.player_id);

    // Persist ban to database and log the action
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        let _ = db.create_ban(body.player_id, Some(body.world_id), "Banned by admin", None).await;
        let _ = db
            .insert_audit_log(
                "admin",
                "ban_player",
                Some(&body.player_id.to_string()),
                Some(&serde_json::json!({ "world_id": body.world_id })),
                None,
            )
            .await;
    }

    // Also kick them if currently connected
    let kicked = state.kick_player(&body.player_id).await;
    world.remove_player(&body.player_id).await;

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "banned": true,
            "player_id": body.player_id,
            "world_id": body.world_id,
            "also_kicked": kicked,
        })),
    )
}

pub(crate) async fn admin_unban_player(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<BanRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let world = match state.get_world(&body.world_id).await {
        Some(w) => w,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "World not found" })),
            );
        }
    };

    let was_banned = world.banned_players.write().await.remove(&body.player_id);

    // Remove from database and log the action
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        let _ = db.remove_ban(body.player_id, Some(body.world_id)).await;
        let _ = db
            .insert_audit_log(
                "admin",
                "unban_player",
                Some(&body.player_id.to_string()),
                Some(&serde_json::json!({ "world_id": body.world_id })),
                None,
            )
            .await;
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "unbanned": was_banned,
            "player_id": body.player_id,
            "world_id": body.world_id,
        })),
    )
}

// ── Debug Endpoint ──────────────────────────────────────────────────────

pub(crate) async fn admin_debug_world(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    match state.get_world(&world_id).await {
        Some(world) => {
            let w = world.world.lock().await;
            let tick = w.current_tick();
            let speed = format!("{:?}", w.speed());

            // Corporation summaries
            let corps: Vec<serde_json::Value> = w.corporations.keys().map(|&cid| {
                let fin = w.financials.get(&cid);
                let nodes = w.corp_infra_nodes.get(&cid).map(|n| n.len()).unwrap_or(0);
                serde_json::json!({
                    "corp_id": cid,
                    "name": w.corporations.get(&cid).map(|c| c.name.as_str()).unwrap_or("?"),
                    "cash": fin.map(|f| f.cash),
                    "revenue": fin.map(|f| f.revenue_per_tick),
                    "cost": fin.map(|f| f.cost_per_tick),
                    "debt": fin.map(|f| f.debt),
                    "nodes": nodes,
                })
            }).collect();

            // Connected players in this world
            let players = state.players.read().await;
            let world_players: Vec<serde_json::Value> = players.values()
                .filter(|p| p.world_id == Some(world_id))
                .map(|p| serde_json::json!({
                    "id": p.id,
                    "username": p.username,
                    "corp_id": p.corp_id,
                    "is_guest": p.is_guest,
                }))
                .collect();

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "world_id": world_id,
                    "world_name": world.name,
                    "tick": tick,
                    "speed": speed,
                    "tick_rate_ms": world.tick_rate_ms,
                    "broadcast_subscribers": world.broadcast_tx.receiver_count(),
                    "corporations": corps,
                    "connected_players": world_players,
                    "entity_counts": {
                        "corporations": w.corporations.len(),
                        "infra_nodes": w.infra_nodes.len(),
                        "infra_edges": w.infra_edges.len(),
                        "regions": w.regions.len(),
                        "cities": w.cities.len(),
                    },
                })),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "World not found" })),
        ),
    }
}

// ── Admin - Templates ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct CreateTemplateRequest {
    name: String,
    description: String,
    icon: String,
    config_defaults: serde_json::Value,
    config_bounds: serde_json::Value,
    max_instances: Option<i32>,
    enabled: Option<bool>,
    sort_order: Option<i32>,
}

pub(crate) async fn admin_create_template(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateTemplateRequest>,
) -> impl IntoResponse {
    if let Err(resp) = extract_admin_claims(&headers) {
        return resp;
    }

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db
            .create_world_template(
                &req.name,
                &req.description,
                &req.icon,
                &req.config_defaults,
                &req.config_bounds,
                req.max_instances.unwrap_or(5),
                req.enabled.unwrap_or(true),
                req.sort_order.unwrap_or(0),
            )
            .await
        {
            Ok(id) => {
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({ "id": id, "status": "created" })),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("Database error: {e}") })),
                );
            }
        }
    }

    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": "Templates require database" })),
    )
}

pub(crate) async fn admin_list_templates(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) = extract_admin_claims(&headers) {
        return resp;
    }

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.list_world_templates(false).await {
            Ok(templates) => {
                let result: Vec<serde_json::Value> = templates
                    .into_iter()
                    .map(|t| {
                        serde_json::json!({
                            "id": t.id,
                            "name": t.name,
                            "description": t.description,
                            "icon": t.icon,
                            "config_defaults": t.config_defaults,
                            "config_bounds": t.config_bounds,
                            "max_instances": t.max_instances,
                            "enabled": t.enabled,
                            "sort_order": t.sort_order,
                        })
                    })
                    .collect();
                return (StatusCode::OK, Json(serde_json::json!(result)));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("Database error: {e}") })),
                );
            }
        }
    }

    (StatusCode::OK, Json(serde_json::json!([])))
}

pub(crate) async fn admin_update_template(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateTemplateRequest>,
) -> impl IntoResponse {
    if let Err(resp) = extract_admin_claims(&headers) {
        return resp;
    }

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db
            .update_world_template(
                id,
                &req.name,
                &req.description,
                &req.icon,
                &req.config_defaults,
                &req.config_bounds,
                req.max_instances.unwrap_or(5),
                req.enabled.unwrap_or(true),
                req.sort_order.unwrap_or(0),
            )
            .await
        {
            Ok(true) => {
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({ "status": "updated" })),
                );
            }
            Ok(false) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "Template not found" })),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("Database error: {e}") })),
                );
            }
        }
    }

    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": "Templates require database" })),
    )
}

pub(crate) async fn admin_delete_template(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(resp) = extract_admin_claims(&headers) {
        return resp;
    }

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.delete_world_template(id).await {
            Ok(true) => {
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({ "status": "deleted" })),
                );
            }
            Ok(false) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "Template not found" })),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("Database error: {e}") })),
                );
            }
        }
    }

    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": "Templates require database" })),
    )
}

// ── Admin - Enhanced Bans, Audit, Reset Queue, Metrics ────────────

pub(crate) async fn admin_list_bans(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) = extract_admin_claims(&headers) {
        return resp;
    }

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.list_bans().await {
            Ok(bans) => {
                let result: Vec<serde_json::Value> = bans
                    .into_iter()
                    .map(|b| {
                        serde_json::json!({
                            "id": b.id,
                            "account_id": b.account_id,
                            "username": b.username,
                            "world_id": b.world_id,
                            "reason": b.reason,
                            "banned_at": b.banned_at.to_rfc3339(),
                            "expires_at": b.expires_at.map(|t| t.to_rfc3339()),
                        })
                    })
                    .collect();
                return (StatusCode::OK, Json(serde_json::json!(result)));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("Database error: {e}") })),
                );
            }
        }
    }

    (StatusCode::OK, Json(serde_json::json!([])))
}

pub(crate) async fn admin_list_reset_queue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) = extract_admin_claims(&headers) {
        return resp;
    }

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.list_pending_reset_requests().await {
            Ok(requests) => {
                let result: Vec<serde_json::Value> = requests
                    .into_iter()
                    .map(|r| {
                        serde_json::json!({
                            "id": r.id,
                            "account_id": r.account_id,
                            "username": r.username,
                            "status": r.status,
                            "created_at": r.created_at.to_rfc3339(),
                        })
                    })
                    .collect();
                return (StatusCode::OK, Json(serde_json::json!(result)));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("Database error: {e}") })),
                );
            }
        }
    }

    (StatusCode::OK, Json(serde_json::json!([])))
}

#[derive(Deserialize)]
pub(crate) struct ResolveResetRequest {
    request_id: Uuid,
}

pub(crate) async fn admin_resolve_reset(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ResolveResetRequest>,
) -> impl IntoResponse {
    if let Err(resp) = extract_admin_claims(&headers) {
        return resp;
    }

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        // Generate a temporary password
        let temp_password = crate::state::generate_invite_code(); // reuse random code generator
        let hash = match auth::hash_password(&temp_password) {
            Ok(h) => h,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("Failed to hash password: {e}") })),
                );
            }
        };

        // First look up the pending request to get the account_id
        let pending = match db.list_pending_reset_requests().await {
            Ok(reqs) => reqs.into_iter().find(|r| r.id == req.request_id),
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("Database error: {e}") })),
                );
            }
        };

        let pending = match pending {
            Some(p) => p,
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "Reset request not found or already resolved" })),
                );
            }
        };

        // Update the password
        if let Err(e) = db.update_password(pending.account_id, &hash).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to update password: {e}") })),
            );
        }

        // Mark the request as resolved
        if let Err(e) = db.resolve_reset_request(req.request_id, "admin").await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Database error: {e}") })),
            );
        }

        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "resolved",
                "temp_password": temp_password,
            })),
        );
    }

    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": "Requires database" })),
    )
}

/// Real-time server and world metrics
pub(crate) async fn admin_metrics(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) = extract_admin_claims(&headers) {
        return resp;
    }

    let worlds = state.worlds.read().await;
    let mut world_metrics = Vec::new();
    for instance in worlds.values() {
        let w = instance.world.lock().await;
        world_metrics.push(serde_json::json!({
            "id": instance.id,
            "name": instance.name,
            "tick": w.current_tick(),
            "speed": w.speed(),
            "player_count": instance.player_count().await,
            "max_players": instance.max_players,
            "config": {
                "starting_era": instance.config.starting_era,
                "map_size": instance.config.map_size,
                "ai_corporations": instance.config.ai_corporations,
                "sandbox": instance.config.sandbox,
            },
            "last_tick_us": instance.last_tick_duration_us.load(std::sync::atomic::Ordering::Relaxed),
            "avg_tick_us": instance.avg_tick_duration_us.load(std::sync::atomic::Ordering::Relaxed),
            "entity_count": w.corporations.len() + w.infra_nodes.len() + w.infra_edges.len(),
            "broadcast_subscribers": instance.broadcast_tx.receiver_count(),
        }));
    }
    let world_count = world_metrics.len();
    drop(worlds);

    let online_count = state.online_players.read().await.len();
    let memory_estimate_bytes = state.memory_usage_estimate().await;

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "worlds": world_metrics,
            "server": {
                "uptime_secs": state.uptime_secs(),
                "connected_players": online_count,
                "world_count": world_count,
                "memory_estimate_bytes": memory_estimate_bytes,
            },
        })),
    )
}
