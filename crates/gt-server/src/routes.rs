use std::sync::Arc;

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
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
        // WebSocket endpoint
        .route("/ws", get(ws_upgrade))
        // Server info
        .route("/api/info", get(server_info))
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
