use std::sync::Arc;

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use gt_common::types::WorldConfig;

use crate::auth;
use crate::state::AppState;
use crate::tick;
use crate::ws;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health))
        // Auth REST endpoints
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        // World management
        .route("/api/worlds", get(list_worlds))
        .route("/api/worlds", post(create_world))
        .route("/api/worlds/{world_id}", get(get_world))
        // Cloud saves
        .route("/api/saves", get(list_saves).post(upload_save))
        .route("/api/saves/{slot}", get(download_save).delete(delete_save))
        // WebSocket endpoint
        .route("/ws", get(ws_upgrade))
        // Server info
        .route("/api/info", get(server_info))
        // Admin endpoints
        .route("/api/admin/players", get(admin_list_players))
        .route("/api/admin/kick", post(admin_kick_player))
        .route("/api/admin/pause", post(admin_pause_world))
        .route("/api/admin/audit", get(admin_audit_log))
        .with_state(state)
}

// ── Health ─────────────────────────────────────────────────────────────────

async fn health() -> &'static str {
    "ok"
}

// ── Auth Endpoints ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct RegisterRequest {
    username: String,
    password: String,
    email: String,
}

async fn register(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterRequest>,
) -> impl IntoResponse {
    if body.username.len() < 3 || body.username.len() > 64 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Username must be 3-64 characters" })),
        );
    }
    if body.password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Password must be at least 8 characters" })),
        );
    }

    let password_hash = match auth::hash_password(&body.password) {
        Ok(h) => h,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Internal error" })),
            );
        }
    };

    match state
        .register_account(body.username.clone(), Some(body.email), password_hash)
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

            let refresh_token =
                auth::generate_refresh_token(&state.auth_config, account.id, &account.username)
                    .unwrap_or_default();

            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "player_id": account.id,
                    "username": account.username,
                    "access_token": access_token,
                    "refresh_token": refresh_token,
                })),
            )
        }
        Err(e) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> impl IntoResponse {
    let account = match state.get_account(&body.username).await {
        Some(a) => a,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Invalid credentials" })),
            );
        }
    };

    match auth::verify_password(&body.password, &account.password_hash) {
        Ok(true) => {}
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Invalid credentials" })),
            );
        }
    }

    let access_token =
        auth::generate_access_token(&state.auth_config, account.id, &account.username, false)
            .unwrap_or_default();

    let refresh_token =
        auth::generate_refresh_token(&state.auth_config, account.id, &account.username)
            .unwrap_or_default();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "player_id": account.id,
            "username": account.username,
            "access_token": access_token,
            "refresh_token": refresh_token,
        })),
    )
}

// ── World Endpoints ────────────────────────────────────────────────────────

async fn list_worlds(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let worlds = state.list_worlds().await;
    Json(worlds)
}

#[derive(Deserialize)]
struct CreateWorldRequest {
    name: String,
    #[serde(default)]
    config: Option<WorldConfig>,
    #[serde(default = "default_max_players")]
    max_players: u32,
}

fn default_max_players() -> u32 {
    8
}

async fn create_world(
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
        tick::spawn_world_tick_loop(world, state.db.clone());
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

async fn get_world(
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

// ── Cloud Saves ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SaveUploadRequest {
    slot: i32,
    name: String,
    tick: i64,
    #[serde(default)]
    config_json: serde_json::Value,
}

async fn upload_save(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SaveUploadRequest>,
) -> impl IntoResponse {
    let _ = &body; // used below in postgres path
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        // TODO: Extract account_id from JWT auth header in production
        let account_id = Uuid::new_v4();
        match db
            .save_cloud(
                account_id,
                body.slot,
                &body.name,
                &[],
                body.tick,
                &body.config_json,
            )
            .await
        {
            Ok(save_id) => {
                return (
                    StatusCode::CREATED,
                    Json(serde_json::json!({
                        "id": save_id,
                        "slot": body.slot,
                        "success": true,
                    })),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("Save failed: {e}") })),
                );
            }
        }
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({ "error": "Database not available" })),
    )
}

async fn list_saves(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let _ = &state;
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        let account_id = Uuid::new_v4(); // Placeholder
        match db.list_cloud_saves(account_id).await {
            Ok(saves) => {
                let list: Vec<serde_json::Value> = saves
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "slot": s.slot,
                            "name": s.name,
                            "tick": s.tick,
                            "size_bytes": s.size_bytes,
                            "created_at": s.created_at.to_rfc3339(),
                        })
                    })
                    .collect();
                return (StatusCode::OK, Json(serde_json::json!({ "saves": list })));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("{e}") })),
                );
            }
        }
    }

    (StatusCode::OK, Json(serde_json::json!({ "saves": [] })))
}

async fn download_save(
    State(state): State<Arc<AppState>>,
    Path(slot): Path<i32>,
) -> impl IntoResponse {
    let _ = (&state, slot);
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        let account_id = Uuid::new_v4(); // Placeholder
        match db.load_cloud_save(account_id, slot).await {
            Ok(Some(data)) => {
                return (StatusCode::OK, data).into_response();
            }
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "Save not found" })),
                )
                    .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("{e}") })),
                )
                    .into_response();
            }
        }
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({ "error": "Database not available" })),
    )
        .into_response()
}

async fn delete_save(
    State(state): State<Arc<AppState>>,
    Path(slot): Path<i32>,
) -> impl IntoResponse {
    let _ = (&state, slot);
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        let account_id = Uuid::new_v4(); // Placeholder
        match db.delete_cloud_save(account_id, slot).await {
            Ok(true) => {
                return (StatusCode::OK, Json(serde_json::json!({ "deleted": true })));
            }
            Ok(false) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "Save not found" })),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("{e}") })),
                );
            }
        }
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({ "error": "Database not available" })),
    )
}

// ── WebSocket ──────────────────────────────────────────────────────────────

async fn ws_upgrade(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws::handle_socket(socket, state))
}

// ── Server Info ────────────────────────────────────────────────────────────

async fn server_info(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let worlds = state.worlds.read().await;
    let players = state.players.read().await;
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "active_worlds": worlds.len(),
        "connected_players": players.len(),
    }))
}

// ── Admin Endpoints ───────────────────────────────────────────────────────

const DEFAULT_ADMIN_KEY: &str = "globaltelco-dev-admin-key";

/// Validate the admin key from the `X-Admin-Key` request header.
/// Returns Ok(()) on success, or an error response tuple on failure.
fn extract_admin_claims(
    headers: &HeaderMap,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let expected_key =
        std::env::var("ADMIN_KEY").unwrap_or_else(|_| DEFAULT_ADMIN_KEY.to_string());

    match headers.get("X-Admin-Key").and_then(|v| v.to_str().ok()) {
        Some(key) if key == expected_key => Ok(()),
        _ => Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Invalid or missing admin key" })),
        )),
    }
}

async fn admin_list_players(
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
struct KickRequest {
    player_id: Uuid,
}

async fn admin_kick_player(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<KickRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let kicked = state.kick_player(&body.player_id).await;
    if kicked {
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
struct PauseRequest {
    world_id: Uuid,
}

async fn admin_pause_world(
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

async fn admin_audit_log(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let log = state.get_audit_log().await;
    (StatusCode::OK, Json(serde_json::json!({ "audit_log": log })))
}
