use std::sync::Arc;
use std::sync::atomic::Ordering;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use gt_common::types::{GameSpeed, WorldConfig};

use crate::state::AppState;
use crate::tick;

use super::super::extract_admin_claims;
use super::super::worlds::{default_max_players, normalize_config};

// ── Pause ─────────────────────────────────────────────────────────────

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

// ── Create World ──────────────────────────────────────────────────────

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

// ── Delete World ──────────────────────────────────────────────────────

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

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        // Even if not in memory, try to archive it in DB
        let _ = db.set_world_status(world_id, "archived").await;

        let details = serde_json::json!({ "kicked_players": kicked_players.len(), "in_memory": removed });
        let _ = db
            .insert_audit_log(
                "admin",
                "delete_world",
                Some(&world_id.to_string()),
                Some(&details),
                None,
            )
            .await;
    }

    if removed {
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "deleted": true,
                "world_id": world_id,
                "kicked_players": kicked_players.len(),
            })),
        )
    } else {
        // If it wasn't in memory but we archived it in DB (above), still return success
        // but with a different flag
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "deleted": true,
                "world_id": world_id,
                "in_memory": false,
                "message": "World was already stopped or not found in memory, but has been archived in database."
            })),
        )
    }
}

// ── Purge Worlds ──────────────────────────────────────────────────────

pub(crate) async fn admin_purge_worlds(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.purge_archived_worlds().await {
            Ok(count) => {
                let details = serde_json::json!({ "purged_count": count });
                let _ = db
                    .insert_audit_log("admin", "purge_worlds", None, Some(&details), None)
                    .await;
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({ "purged": true, "count": count })),
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
        Json(serde_json::json!({ "error": "Purge requires database" })),
    )
}

// ── Set Speed ─────────────────────────────────────────────────────────

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

// ── Broadcast ─────────────────────────────────────────────────────────

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

// ── World Chat History ────────────────────────────────────────────────

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

// ── Transfer World Ownership ──────────────────────────────────────────

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

// ── World Speed Votes ─────────────────────────────────────────────────

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
