use std::sync::Arc;

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use gt_common::types::WorldConfig;

use crate::state::AppState;
use crate::tick;
use crate::ws;

use super::AuthClaims;

// ── World Endpoints ────────────────────────────────────────────────────────

pub(crate) async fn list_worlds(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let worlds = state.list_worlds().await;
    Json(worlds)
}

#[derive(Deserialize)]
pub(crate) struct CreateWorldRequest {
    name: String,
    #[serde(default)]
    config: Option<WorldConfig>,
    #[serde(default = "default_max_players")]
    max_players: u32,
}

pub(super) fn default_max_players() -> u32 {
    8
}

pub(crate) async fn create_world(
    _: AuthClaims,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateWorldRequest>,
) -> impl IntoResponse {
    let config = body.config.unwrap_or_default();
    let world_id = state
        .create_world(body.name.clone(), config, body.max_players)
        .await;

    // Start the tick loop for this world
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

pub(crate) async fn get_world(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.get_world(&world_id).await {
        Some(world) => {
            let w = world.world.lock().await;
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "id": world.id,
                    "name": world.name,
                    "tick": w.current_tick(),
                    "player_count": world.player_count().await,
                    "max_players": world.max_players,
                })),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "World not found" })),
        ),
    }
}

// ── WebSocket ──────────────────────────────────────────────────────────────

pub(crate) async fn ws_upgrade(
    ws: WebSocketUpgrade,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let ip = addr.ip();
    ws.on_upgrade(move |socket| ws::handle_socket(socket, state, ip))
}

// ── World Catalog ─────────────────────────────────────────────────────────

/// List enabled world templates (public catalog)
pub(crate) async fn list_catalog(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.list_world_templates(true).await {
            Ok(templates) => {
                let mut result = Vec::new();
                for t in templates {
                    let instance_count = db.count_template_instances(t.id).await.unwrap_or(0);
                    result.push(serde_json::json!({
                        "id": t.id,
                        "name": t.name,
                        "description": t.description,
                        "icon": t.icon,
                        "config_defaults": t.config_defaults,
                        "config_bounds": t.config_bounds,
                        "max_instances": t.max_instances,
                        "current_instances": instance_count,
                    }));
                }
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

/// Get a single catalog template
pub(crate) async fn get_catalog_template(
    State(state): State<Arc<AppState>>,
    Path(template_id): Path<Uuid>,
) -> impl IntoResponse {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.get_template(template_id).await {
            Ok(Some(t)) => {
                let instance_count = db.count_template_instances(t.id).await.unwrap_or(0);
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "id": t.id,
                        "name": t.name,
                        "description": t.description,
                        "icon": t.icon,
                        "config_defaults": t.config_defaults,
                        "config_bounds": t.config_bounds,
                        "max_instances": t.max_instances,
                        "current_instances": instance_count,
                        "enabled": t.enabled,
                    })),
                );
            }
            Ok(None) => {
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
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "Templates require database" })),
    )
}

#[derive(Deserialize)]
pub(crate) struct CreateFromTemplateRequest {
    template_id: Uuid,
    name: String,
    max_players: Option<u32>,
    config_overrides: Option<serde_json::Value>,
}

/// Validate config overrides are within template bounds, merge with defaults
fn validate_config_within_bounds(
    overrides: &serde_json::Value,
    defaults: &serde_json::Value,
    bounds: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut merged = defaults.clone();

    if let (Some(overrides_obj), Some(merged_obj)) =
        (overrides.as_object(), merged.as_object_mut())
    {
        let empty_map = serde_json::Map::new();
        let bounds_obj = bounds.as_object().unwrap_or(&empty_map);

        for (key, value) in overrides_obj {
            if let Some(bound) = bounds_obj.get(key) {
                // Check numeric bounds
                if let (Some(val), Some(min), Some(max)) =
                    (value.as_f64(), bound.get("min").and_then(|v| v.as_f64()), bound.get("max").and_then(|v| v.as_f64()))
                {
                    if val < min || val > max {
                        return Err(format!(
                            "Field '{}' value {} is out of bounds [{}, {}]",
                            key, val, min, max
                        ));
                    }
                }
                // Check allowed list
                if let Some(allowed) = bound.get("allowed").and_then(|v| v.as_array()) {
                    if !allowed.contains(value) {
                        return Err(format!(
                            "Field '{}' value {:?} is not in allowed list",
                            key, value
                        ));
                    }
                }
                merged_obj.insert(key.clone(), value.clone());
            } else {
                return Err(format!("Field '{}' is not customizable", key));
            }
        }
    }

    Ok(merged)
}

/// Create a world from a catalog template
pub(crate) async fn create_world_from_template(
    State(state): State<Arc<AppState>>,
    AuthClaims(player_id): AuthClaims,
    Json(req): Json<CreateFromTemplateRequest>,
) -> impl IntoResponse {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        // Get the template
        let template = match db.get_template(req.template_id).await {
            Ok(Some(t)) => t,
            Ok(None) => {
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
        };

        if !template.enabled {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Template is disabled" })),
            );
        }

        // Check instance limit
        let instance_count = db.count_template_instances(req.template_id).await.unwrap_or(0);
        if instance_count >= template.max_instances as i64 {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({ "error": "Maximum instances for this template reached" })),
            );
        }

        // Validate and merge config overrides
        let config_json = if let Some(ref overrides) = req.config_overrides {
            match validate_config_within_bounds(overrides, &template.config_defaults, &template.config_bounds) {
                Ok(merged) => merged,
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({ "error": e })),
                    );
                }
            }
        } else {
            template.config_defaults.clone()
        };

        // Parse into WorldConfig
        let config: WorldConfig = match serde_json::from_value(config_json) {
            Ok(c) => c,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": format!("Invalid config: {e}") })),
                );
            }
        };

        let max_players = req.max_players.unwrap_or(8);
        let invite_code = crate::state::generate_invite_code();

        let world_id = state
            .create_world_from_template(
                req.name,
                config,
                max_players,
                Some(req.template_id),
                Some(invite_code.clone()),
            )
            .await;

        // Start tick loop
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

        // Record template + creator in DB
        let _ = db.set_world_template_id(world_id, req.template_id).await;
        let _ = db.set_world_creator(world_id, player_id).await;
        let _ = db.set_world_invite_code(world_id, &invite_code).await;

        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "world_id": world_id,
                "invite_code": invite_code,
            })),
        );
    }

    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": "Templates require database" })),
    )
}

/// Look up a world by invite code
pub(crate) async fn get_world_by_invite(
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
) -> impl IntoResponse {
    // First check in-memory active worlds
    let worlds = state.worlds.read().await;
    for instance in worlds.values() {
        if instance.invite_code.as_deref() == Some(code.as_str()) {
            let w = instance.world.lock().await;
            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "id": instance.id,
                    "name": instance.name,
                    "player_count": instance.player_count().await,
                    "max_players": instance.max_players,
                    "tick": w.current_tick(),
                    "speed": w.speed(),
                    "invite_code": code,
                })),
            );
        }
    }
    drop(worlds);

    // Fallback: check database for persisted worlds
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        if let Ok(Some(row)) = db.get_world_by_invite_code(&code).await {
            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "id": row.id,
                    "name": row.name,
                    "player_count": 0,
                    "max_players": row.max_players,
                    "tick": row.current_tick,
                    "speed": row.speed,
                    "invite_code": code,
                    "persisted": true,
                })),
            );
        }
    }

    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "No active world found with that invite code" })),
    )
}
