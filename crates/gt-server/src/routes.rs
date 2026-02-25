use std::sync::Arc;

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use tower_http::services::ServeDir;
use uuid::Uuid;

use gt_common::types::{GameSpeed, WorldConfig};

use crate::auth;
use crate::state::AppState;
use crate::tick;
use crate::ws;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

pub fn create_router(state: Arc<AppState>, tile_dir: Option<String>) -> Router {
    let mut router = Router::new()
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
        .route("/api/admin/health", get(admin_health))
        .route("/api/admin/worlds", post(admin_create_world))
        .route("/api/admin/worlds/{world_id}", delete(admin_delete_world))
        .route("/api/admin/worlds/{world_id}/speed", post(admin_set_speed))
        .route("/api/admin/broadcast", post(admin_broadcast))
        .route("/api/admin/debug/{world_id}", get(admin_debug_world))
        .route("/api/admin/ban", post(admin_ban_player))
        .route("/api/admin/unban", post(admin_unban_player));

    // Mount tile serving if TILE_DIR is configured
    if let Some(ref dir) = tile_dir {
        let serve_dir = ServeDir::new(dir)
            .append_index_html_on_directories(false);
        router = router.nest_service("/tiles", serve_dir);
    } else {
        router = router.route("/tiles/{*path}", get(tiles_not_configured));
    }

    router.with_state(state)
}

/// Returns 404 when tile serving is not configured
async fn tiles_not_configured() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        [("content-type", "application/json")],
        r#"{"error":"Tile serving not configured. Set TILE_DIR environment variable."}"#,
    )
}

/// Extractor for authenticated user claims
pub struct AuthClaims(pub Uuid);

impl<S> FromRequestParts<S> for AuthClaims
where
    S: Send + Sync,
    Arc<AppState>: FromRef<S>,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = Arc::<AppState>::from_ref(state);
        
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Missing authorization header" })),
            ))?;

        if !auth_header.starts_with("Bearer ") {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Invalid authorization header" })),
            ));
        }

        let token = &auth_header[7..];
        let claims = auth::validate_token(&app_state.auth_config, token).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Invalid or expired token" })),
            )
        })?;

        let player_id = Uuid::parse_str(&claims.sub).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Invalid player ID in token" })),
            )
        })?;

        Ok(AuthClaims(player_id))
    }
}

use axum::extract::FromRef;

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
#[allow(dead_code)]
struct SaveUploadRequest {
    slot: i32,
    name: String,
    tick: i64,
    #[serde(default)]
    config_json: serde_json::Value,
}

/// Maximum cloud save size in bytes (50 MB)
#[allow(dead_code)]
const MAX_SAVE_SIZE: usize = 50_000_000;

#[allow(unused_variables)]
async fn upload_save(
    AuthClaims(account_id): AuthClaims,
    State(state): State<Arc<AppState>>,
    Json(body): Json<SaveUploadRequest>,
) -> impl IntoResponse {
    let _ = &body; // used below in postgres path
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
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

#[allow(unused_variables)]
async fn list_saves(
    AuthClaims(account_id): AuthClaims,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let _ = &state;
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
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

#[allow(unused_variables)]
async fn download_save(
    AuthClaims(account_id): AuthClaims,
    State(state): State<Arc<AppState>>,
    Path(slot): Path<i32>,
) -> impl IntoResponse {
    let _ = (&state, slot);
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
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

#[allow(unused_variables)]
async fn delete_save(
    AuthClaims(account_id): AuthClaims,
    State(state): State<Arc<AppState>>,
    Path(slot): Path<i32>,
) -> impl IntoResponse {
    let _ = (&state, slot);
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
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

async fn ws_upgrade(
    ws: WebSocketUpgrade,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let ip = addr.ip();
    ws.on_upgrade(move |socket| ws::handle_socket(socket, state, ip))
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

/// Validate the admin key from the `X-Admin-Key` request header.
/// ADMIN_KEY env var is required — if unset, all admin endpoints return 503.
/// Uses constant-time comparison to prevent timing attacks.
fn extract_admin_claims(
    headers: &HeaderMap,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let expected = std::env::var("ADMIN_KEY").map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Admin not configured" })),
        )
    })?;

    if expected.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Admin not configured" })),
        ));
    }

    match headers.get("X-Admin-Key").and_then(|v| v.to_str().ok()) {
        Some(key) => {
            use subtle::ConstantTimeEq;
            if key.as_bytes().ct_eq(expected.as_bytes()).into() {
                Ok(())
            } else {
                Err((
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({ "error": "Invalid admin key" })),
                ))
            }
        }
        None => Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Missing admin key" })),
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

async fn admin_health(
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
struct AdminCreateWorldRequest {
    name: String,
    #[serde(default)]
    config: Option<WorldConfig>,
    #[serde(default = "default_max_players")]
    max_players: u32,
}

async fn admin_create_world(
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

async fn admin_delete_world(
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
struct SetSpeedRequest {
    speed: String,
}

async fn admin_set_speed(
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
struct BroadcastRequest {
    message: String,
    #[serde(default)]
    world_id: Option<Uuid>,
}

async fn admin_broadcast(
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
struct BanRequest {
    world_id: Uuid,
    player_id: Uuid,
}

async fn admin_ban_player(
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

    // Add to ban list
    world.banned_players.write().await.insert(body.player_id);

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

async fn admin_unban_player(
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

async fn admin_debug_world(
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
