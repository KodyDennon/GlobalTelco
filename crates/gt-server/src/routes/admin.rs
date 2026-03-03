use std::sync::Arc;
use std::sync::atomic::Ordering;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use gt_common::types::{EntityId, GameSpeed, WorldConfig};

use crate::auth;
use crate::state::AppState;
use crate::tick;

use super::extract_admin_claims;
use super::worlds::{default_max_players, normalize_config};

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

#[derive(Deserialize, Default)]
pub(crate) struct AuditLogQuery {
    #[serde(default = "default_audit_limit")]
    limit: Option<i64>,
    #[serde(default)]
    offset: Option<i64>,
    #[serde(default)]
    actor: Option<String>,
}

fn default_audit_limit() -> Option<i64> {
    Some(100)
}

pub(crate) async fn admin_audit_log(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<AuditLogQuery>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    // Use DB-backed paginated audit log when available
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.query_audit_log(limit, offset, params.actor.as_deref()).await {
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

    // Fallback to in-memory audit log (apply pagination)
    let log = state.get_audit_log().await;
    let total = log.len();
    let start = (offset as usize).min(total);
    let end = (start + limit as usize).min(total);
    let page = &log[start..end];
    (StatusCode::OK, Json(serde_json::json!({ "audit_log": page, "total": total })))
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

    let config = normalize_config(body.config.unwrap_or_default());
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

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        let _ = db
            .insert_audit_log(
                "admin",
                "create_world",
                Some(&world_id.to_string()),
                Some(&serde_json::json!({ "name": body.name })),
                None,
            )
            .await;
    }

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "world_id": world_id,
            "name": body.name,
        })),
    )
}

// ── Server Limits ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct SetLimitsRequest {
    max_active_worlds: Option<u64>,
    max_worlds_per_player: Option<u64>,
}

pub(crate) async fn admin_get_limits(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let active = state.active_world_count().await;
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "max_active_worlds": state.max_active_worlds.load(Ordering::Relaxed),
            "max_worlds_per_player": state.max_worlds_per_player.load(Ordering::Relaxed),
            "active_world_count": active,
        })),
    )
}

pub(crate) async fn admin_set_limits(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetLimitsRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    if let Some(v) = body.max_active_worlds {
        state.max_active_worlds.store(v, Ordering::Relaxed);
    }
    if let Some(v) = body.max_worlds_per_player {
        state.max_worlds_per_player.store(v, Ordering::Relaxed);
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "max_active_worlds": state.max_active_worlds.load(Ordering::Relaxed),
            "max_worlds_per_player": state.max_worlds_per_player.load(Ordering::Relaxed),
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

            #[cfg(feature = "postgres")]
            if let Some(db) = state.db.as_ref() {
                let _ = db
                    .insert_audit_log(
                        "admin",
                        "set_speed",
                        Some(&world_id.to_string()),
                        Some(&serde_json::json!({ "speed": body.speed })),
                        None,
                    )
                    .await;
            }

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

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        let _ = db
            .insert_audit_log(
                "admin",
                "broadcast",
                body.world_id.as_ref().map(|w| w.to_string()).as_deref(),
                Some(&serde_json::json!({ "message": body.message })),
                None,
            )
            .await;
    }

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
    account_id: Uuid,
    reason: String,
    #[serde(default)]
    world_id: Option<Uuid>,
    #[serde(default)]
    expires_at: Option<String>,
}

pub(crate) async fn admin_ban_player(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<BanRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    // Parse optional expiry
    let expires_at_dt = body.expires_at.as_deref().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&chrono::Utc))
    });

    if let Some(wid) = body.world_id {
        // World-specific ban
        let world = match state.get_world(&wid).await {
            Some(w) => w,
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "World not found" })),
                );
            }
        };
        world.banned_players.write().await.insert(body.account_id);

        #[cfg(feature = "postgres")]
        if let Some(db) = state.db.as_ref() {
            let _ = db.create_ban(body.account_id, Some(wid), &body.reason, expires_at_dt).await;
            let _ = db
                .insert_audit_log(
                    "admin",
                    "ban_player",
                    Some(&body.account_id.to_string()),
                    Some(&serde_json::json!({ "world_id": wid, "reason": body.reason })),
                    None,
                )
                .await;
        }

        // Also kick them if currently connected
        let kicked = state.kick_player(&body.account_id).await;
        world.remove_player(&body.account_id).await;

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "banned": true,
                "account_id": body.account_id,
                "world_id": wid,
                "also_kicked": kicked,
            })),
        )
    } else {
        // Global ban — add to all worlds
        let worlds = state.worlds.read().await;
        for instance in worlds.values() {
            instance.banned_players.write().await.insert(body.account_id);
        }
        drop(worlds);

        #[cfg(feature = "postgres")]
        if let Some(db) = state.db.as_ref() {
            let _ = db.create_ban(body.account_id, None, &body.reason, expires_at_dt).await;
            let _ = db
                .insert_audit_log(
                    "admin",
                    "ban_player_global",
                    Some(&body.account_id.to_string()),
                    Some(&serde_json::json!({ "reason": body.reason })),
                    None,
                )
                .await;
        }

        let kicked = state.kick_player(&body.account_id).await;

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "banned": true,
                "account_id": body.account_id,
                "world_id": null,
                "also_kicked": kicked,
            })),
        )
    }
}

#[derive(Deserialize)]
pub(crate) struct UnbanRequest {
    account_id: Uuid,
    #[serde(default)]
    world_id: Option<Uuid>,
}

pub(crate) async fn admin_unban_player(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<UnbanRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    if let Some(wid) = body.world_id {
        let world = match state.get_world(&wid).await {
            Some(w) => w,
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "World not found" })),
                );
            }
        };

        let was_banned = world.banned_players.write().await.remove(&body.account_id);

        #[cfg(feature = "postgres")]
        if let Some(db) = state.db.as_ref() {
            let _ = db.remove_ban(body.account_id, Some(wid)).await;
            let _ = db
                .insert_audit_log(
                    "admin",
                    "unban_player",
                    Some(&body.account_id.to_string()),
                    Some(&serde_json::json!({ "world_id": wid })),
                    None,
                )
                .await;
        }

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "unbanned": was_banned,
                "account_id": body.account_id,
                "world_id": wid,
            })),
        )
    } else {
        // Global unban — remove from all worlds
        let worlds = state.worlds.read().await;
        let mut was_banned = false;
        for instance in worlds.values() {
            if instance.banned_players.write().await.remove(&body.account_id) {
                was_banned = true;
            }
        }
        drop(worlds);

        #[cfg(feature = "postgres")]
        if let Some(db) = state.db.as_ref() {
            let _ = db.remove_ban(body.account_id, None).await;
            let _ = db
                .insert_audit_log(
                    "admin",
                    "unban_player_global",
                    Some(&body.account_id.to_string()),
                    None,
                    None,
                )
                .await;
        }

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "unbanned": was_banned,
                "account_id": body.account_id,
                "world_id": null,
            })),
        )
    }
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
                let _ = db
                    .insert_audit_log(
                        "admin",
                        "create_template",
                        Some(&id.to_string()),
                        Some(&serde_json::json!({ "name": req.name })),
                        None,
                    )
                    .await;
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
                let _ = db
                    .insert_audit_log(
                        "admin",
                        "update_template",
                        Some(&id.to_string()),
                        Some(&serde_json::json!({ "name": req.name })),
                        None,
                    )
                    .await;
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
                let _ = db
                    .insert_audit_log(
                        "admin",
                        "delete_template",
                        Some(&id.to_string()),
                        None,
                        None,
                    )
                    .await;
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

        let _ = db
            .insert_audit_log(
                "admin",
                "resolve_reset",
                Some(&req.request_id.to_string()),
                Some(&serde_json::json!({ "account_id": pending.account_id })),
                None,
            )
            .await;

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
        let tick_history: Vec<u64> = instance.tick_history.read().await.iter().copied().collect();
        let system_times: std::collections::HashMap<String, u64> = w.system_times.clone();
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
            "max_tick_us": instance.max_tick_us.load(std::sync::atomic::Ordering::Relaxed),
            "p99_tick_us": instance.p99_tick_us.load(std::sync::atomic::Ordering::Relaxed),
            "tick_history": tick_history,
            "system_times": system_times,
            "entity_count": w.corporations.len() + w.infra_nodes.len() + w.infra_edges.len(),
            "broadcast_subscribers": instance.broadcast_tx.receiver_count(),
        }));
    }
    let world_count = world_metrics.len();
    drop(worlds);

    let online_count = state.online_players.read().await.len();
    let memory_estimate_bytes = state.memory_usage_estimate().await;
    let memory_mb = memory_estimate_bytes as f64 / 1_048_576.0;
    let ws_msg_per_sec = state.ws_messages_per_sec().await;

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "worlds": world_metrics,
            "server": {
                "uptime_secs": state.uptime_secs(),
                "connected_players": online_count,
                "world_count": world_count,
                "memory_estimate_bytes": memory_estimate_bytes,
                "memory_mb": memory_mb,
                "ws_messages_per_sec": ws_msg_per_sec,
            },
        })),
    )
}

// ── New Admin Endpoints ─────────────────────────────────────────────────

// 1. List accounts (paginated, searchable)
#[derive(Deserialize, Default)]
pub(crate) struct ListAccountsQuery {
    #[serde(default)]
    search: Option<String>,
    #[serde(default = "default_page")]
    page: Option<i64>,
    #[serde(default = "default_per_page")]
    per_page: Option<i64>,
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    order: Option<String>,
}

fn default_page() -> Option<i64> {
    Some(1)
}

fn default_per_page() -> Option<i64> {
    Some(50)
}

pub(crate) async fn admin_list_accounts(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<ListAccountsQuery>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;
    let sort = params.sort.as_deref().unwrap_or("created_at");
    let order = params.order.as_deref().unwrap_or("desc");

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.list_accounts(params.search.as_deref(), per_page, offset, sort, order).await {
            Ok((accounts, total)) => {
                let result: Vec<serde_json::Value> = accounts
                    .into_iter()
                    .map(|a| {
                        serde_json::json!({
                            "id": a.id,
                            "username": a.username,
                            "email": a.email,
                            "display_name": a.display_name,
                            "avatar_id": a.avatar_id,
                            "auth_provider": a.auth_provider,
                            "is_guest": a.is_guest,
                            "created_at": a.created_at,
                            "last_login": a.last_login,
                            "deleted_at": a.deleted_at,
                        })
                    })
                    .collect();
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "accounts": result,
                        "total": total,
                        "page": page,
                        "per_page": per_page,
                    })),
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

    // Fallback: return in-memory accounts
    let accounts = state.accounts.read().await;
    let list: Vec<serde_json::Value> = accounts
        .values()
        .map(|a| {
            serde_json::json!({
                "id": a.id,
                "username": a.username,
                "email": a.email,
                "display_name": a.display_name,
                "avatar_id": a.avatar_id,
                "auth_provider": a.auth_provider,
                "is_guest": a.is_guest,
            })
        })
        .collect();
    let total = list.len();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "accounts": list,
            "total": total,
            "page": page,
            "per_page": per_page,
        })),
    )
}

// 2. List connections
pub(crate) async fn admin_list_connections(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let players = state.players.read().await;
    let online = state.online_players.read().await;
    let worlds = state.worlds.read().await;

    let connections: Vec<serde_json::Value> = players
        .values()
        .map(|p| {
            let presence = online.get(&p.id);
            let world_name = p.world_id.and_then(|wid| {
                worlds.get(&wid).map(|w| w.name.clone())
            });
            serde_json::json!({
                "id": p.id,
                "username": p.username,
                "world_id": p.world_id,
                "world_name": world_name,
                "corp_id": p.corp_id,
                "is_guest": p.is_guest,
                "is_spectator": p.is_spectator,
                "connected_at": presence.map(|pr| pr.connected_at),
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(serde_json::json!({ "connections": connections })),
    )
}

// 3. World chat history
#[derive(Deserialize, Default)]
pub(crate) struct ChatQuery {
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    before: Option<String>,
}

pub(crate) async fn admin_world_chat(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<Uuid>,
    axum::extract::Query(params): axum::extract::Query<ChatQuery>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    // Verify world exists
    if state.get_world(&world_id).await.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "World not found" })),
        );
    }

    let limit = params.limit.unwrap_or(50).clamp(1, 500);
    let before = params.before.as_deref().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&chrono::Utc))
    });

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.list_chat_messages(world_id, limit, before).await {
            Ok(messages) => {
                let result: Vec<serde_json::Value> = messages
                    .into_iter()
                    .map(|m| {
                        serde_json::json!({
                            "id": m.id,
                            "world_id": m.world_id,
                            "account_id": m.account_id,
                            "username": m.username,
                            "message": m.message,
                            "created_at": m.created_at.to_rfc3339(),
                        })
                    })
                    .collect();
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({ "messages": result })),
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
        StatusCode::OK,
        Json(serde_json::json!({ "messages": [] })),
    )
}

// 4. Assign player to corporation
#[derive(Deserialize)]
pub(crate) struct AssignPlayerRequest {
    player_id: Uuid,
    corp_id: EntityId,
}

pub(crate) async fn admin_assign_player(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<Uuid>,
    Json(body): Json<AssignPlayerRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let world = match state.get_world(&world_id).await {
        Some(w) => w,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "World not found" })),
            );
        }
    };

    // Update the world's players map
    world.players.write().await.insert(body.player_id, body.corp_id);

    // Update the connected player's corp_id if they're online
    if let Some(player) = state.players.write().await.get_mut(&body.player_id) {
        player.corp_id = Some(body.corp_id);
    }

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        let _ = db
            .insert_audit_log(
                "admin",
                "assign_player",
                Some(&body.player_id.to_string()),
                Some(&serde_json::json!({ "world_id": world_id, "corp_id": body.corp_id })),
                None,
            )
            .await;
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "assigned": true,
            "player_id": body.player_id,
            "corp_id": body.corp_id,
            "world_id": world_id,
        })),
    )
}

// 5. Toggle spectator mode
#[derive(Deserialize)]
pub(crate) struct SpectatorRequest {
    player_id: Uuid,
    spectator: bool,
}

pub(crate) async fn admin_toggle_spectator(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<Uuid>,
    Json(body): Json<SpectatorRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    // Verify world exists
    if state.get_world(&world_id).await.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "World not found" })),
        );
    }

    let mut players = state.players.write().await;
    if let Some(player) = players.get_mut(&body.player_id) {
        player.is_spectator = body.spectator;
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "player_id": body.player_id,
                "spectator": body.spectator,
                "world_id": world_id,
            })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Player not found" })),
        )
    }
}

// 6. Transfer world ownership
#[derive(Deserialize)]
pub(crate) struct TransferWorldRequest {
    new_owner_id: Uuid,
}

pub(crate) async fn admin_transfer_world(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<Uuid>,
    Json(body): Json<TransferWorldRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let world = match state.get_world(&world_id).await {
        Some(w) => w,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "World not found" })),
            );
        }
    };

    let prev_owner = world.creator_id.read().await.clone();
    *world.creator_id.write().await = Some(body.new_owner_id);

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        let _ = db
            .insert_audit_log(
                "admin",
                "transfer_world",
                Some(&world_id.to_string()),
                Some(&serde_json::json!({
                    "previous_owner": prev_owner,
                    "new_owner": body.new_owner_id,
                })),
                None,
            )
            .await;
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "world_id": world_id,
            "previous_owner": prev_owner,
            "new_owner_id": body.new_owner_id,
        })),
    )
}

// 7. World speed votes
pub(crate) async fn admin_world_votes(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let world = match state.get_world(&world_id).await {
        Some(w) => w,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "World not found" })),
            );
        }
    };

    let speed_votes = world.speed_votes.read().await;
    let creator_id = world.creator_id.read().await;
    let w = world.world.lock().await;
    let current_speed = format!("{:?}", w.speed());
    drop(w);

    let votes: serde_json::Value = serde_json::to_value(
        speed_votes.iter().map(|(k, v)| (k.to_string(), v.clone())).collect::<std::collections::HashMap<_, _>>()
    ).unwrap_or_default();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "world_id": world_id,
            "votes": votes,
            "current_speed": current_speed,
            "creator_id": *creator_id,
        })),
    )
}

// 8. Server configuration
pub(crate) async fn admin_server_config(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let env_vars = serde_json::json!({
        "ADMIN_KEY": std::env::var("ADMIN_KEY").is_ok(),
        "DATABASE_URL": std::env::var("DATABASE_URL").is_ok(),
        "JWT_SECRET": std::env::var("GT_JWT_SECRET").is_ok(),
        "GITHUB_CLIENT_ID": std::env::var("GITHUB_CLIENT_ID").is_ok(),
        "GITHUB_CLIENT_SECRET": std::env::var("GITHUB_CLIENT_SECRET").is_ok(),
        "TILE_DIR": std::env::var("TILE_DIR").is_ok(),
        "CORS_ORIGIN": std::env::var("CORS_ORIGIN").is_ok(),
        "CORS_ORIGINS": std::env::var("CORS_ORIGINS").is_ok(),
        "R2_ACCOUNT_ID": std::env::var("R2_ACCOUNT_ID").is_ok(),
        "R2_ACCESS_KEY_ID": std::env::var("R2_ACCESS_KEY_ID").is_ok(),
        "R2_SECRET_ACCESS_KEY": std::env::var("R2_SECRET_ACCESS_KEY").is_ok(),
        "R2_BUCKET_NAME": std::env::var("R2_BUCKET_NAME").is_ok(),
    });

    let has_postgres = state.db.is_some();
    let has_r2;
    #[cfg(feature = "r2")]
    {
        has_r2 = state.r2.is_some();
    }
    #[cfg(not(feature = "r2"))]
    {
        has_r2 = false;
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "env_vars": env_vars,
            "database": {
                "connected": has_postgres,
                "pool_size": state.db.as_ref().map(|db| db.pool_size()).unwrap_or(0),
            },
            "features": {
                "postgres": has_postgres,
                "r2": has_r2,
            },
        })),
    )
}
