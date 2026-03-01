use std::sync::Arc;

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use tower_http::services::ServeDir;
use uuid::Uuid;

use gt_common::types::{GameSpeed, WorldConfig};

use crate::auth;
use crate::state::AppState;
use crate::tick;
use crate::ws;

use crate::oauth;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

pub fn create_router(state: Arc<AppState>, tile_dir: Option<String>) -> Router {
    let mut router = Router::new()
        // Health check
        .route("/health", get(health))
        // Auth REST endpoints
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh_token))
        .route("/api/auth/github", get(github_auth_url))
        .route("/api/auth/github/callback", get(github_callback))
        .route("/api/auth/reset-request", post(request_password_reset))
        .route("/api/auth/reset-confirm", post(confirm_password_reset))
        // Profile endpoints
        .route("/api/profile", get(get_own_profile).put(update_profile))
        .route("/api/profile/{player_id}", get(get_player_profile))
        .route("/api/account/delete", post(delete_account))
        .route("/api/avatars", get(list_avatars))
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
        .route("/api/admin/unban", post(admin_unban_player))
        // Phase 2: World Catalog
        .route("/api/catalog", get(list_catalog))
        .route("/api/catalog/{template_id}", get(get_catalog_template))
        .route("/api/worlds/from-template", post(create_world_from_template))
        .route("/api/worlds/by-invite/{code}", get(get_world_by_invite))
        // Phase 2: Admin - Templates
        .route("/api/admin/templates", get(admin_list_templates).post(admin_create_template))
        .route("/api/admin/templates/{id}", put(admin_update_template).delete(admin_delete_template))
        // Phase 2: Admin - Enhanced bans, audit, reset queue, metrics
        .route("/api/admin/bans", get(admin_list_bans))
        .route("/api/admin/reset-queue", get(admin_list_reset_queue))
        .route("/api/admin/reset-resolve", post(admin_resolve_reset))
        .route("/api/admin/metrics", get(admin_metrics))
        // Phase 3: Social system
        .route("/api/friends", get(list_friends))
        .route("/api/friends/request", post(send_friend_request))
        .route("/api/friends/requests", get(list_friend_requests))
        .route("/api/friends/accept", post(accept_friend_request))
        .route("/api/friends/reject", post(reject_friend_request))
        .route("/api/friends/{friend_id}", delete(remove_friend))
        .route("/api/friends/search", get(search_users))
        .route("/api/friends/invite", post(invite_friend_to_world))
        .route("/api/recent-players", get(list_recent_players))
        .route("/api/world-history", get(list_world_history))
        // Per-world leaderboard
        .route("/api/worlds/{world_id}/leaderboard", get(get_world_leaderboard))
        // Account linking
        .route("/api/auth/link-github", post(link_github_account));

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

    // Update last login timestamp in database
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        let _ = db.update_last_login(account.id).await;
    }

    let access_token =
        auth::generate_access_token(&state.auth_config, account.id, &account.username, account.is_guest)
            .unwrap_or_default();

    let refresh_token =
        auth::generate_refresh_token(&state.auth_config, account.id, &account.username)
            .unwrap_or_default();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "player_id": account.id,
            "username": account.username,
            "email": account.email,
            "is_guest": account.is_guest,
            "access_token": access_token,
            "refresh_token": refresh_token,
        })),
    )
}

// ── Token Refresh ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct RefreshTokenRequest {
    refresh_token: String,
}

async fn refresh_token(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RefreshTokenRequest>,
) -> impl IntoResponse {
    match auth::validate_token(&state.auth_config, &body.refresh_token) {
        Ok(claims) => {
            let player_id = match Uuid::parse_str(&claims.sub) {
                Ok(id) => id,
                Err(_) => {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(serde_json::json!({ "error": "Invalid token" })),
                    );
                }
            };

            let access_token = auth::generate_access_token(
                &state.auth_config,
                player_id,
                &claims.username,
                claims.is_guest,
            )
            .unwrap_or_default();

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "access_token": access_token,
                    "player_id": player_id,
                    "username": claims.username,
                })),
            )
        }
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Invalid or expired refresh token" })),
        ),
    }
}

// ── GitHub OAuth ──────────────────────────────────────────────────────────

async fn github_auth_url(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match &state.oauth_config {
        Some(oauth) => {
            let url = format!(
                "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=read:user,user:email",
                oauth.github_client_id, oauth.github_redirect_uri,
            );
            (StatusCode::OK, Json(serde_json::json!({ "url": url })))
        }
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "GitHub OAuth not configured" })),
        ),
    }
}

#[derive(Deserialize)]
struct GitHubCallbackQuery {
    code: String,
}

async fn github_callback(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<GitHubCallbackQuery>,
) -> impl IntoResponse {
    #[cfg(not(feature = "oauth"))]
    {
        let _ = (&state, &query);
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "OAuth feature not enabled" })),
        );
    }

    #[cfg(feature = "oauth")]
    {
        let oauth_config = match &state.oauth_config {
            Some(c) => c,
            None => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({ "error": "GitHub OAuth not configured" })),
                );
            }
        };

        let github_user = match oauth::github_exchange(oauth_config, &query.code).await {
            Ok(u) => u,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": format!("GitHub auth failed: {e}") })),
                );
            }
        };

        // Check if account exists for this GitHub ID
        if let Some(existing) = state.get_account_by_github_id(github_user.id).await {
            let access_token = auth::generate_access_token(
                &state.auth_config,
                existing.id,
                &existing.username,
                false,
            )
            .unwrap_or_default();
            let refresh_token =
                auth::generate_refresh_token(&state.auth_config, existing.id, &existing.username)
                    .unwrap_or_default();

            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "player_id": existing.id,
                    "username": existing.username,
                    "access_token": access_token,
                    "refresh_token": refresh_token,
                    "is_new": false,
                })),
            );
        }

        // Create new account from GitHub
        let username = github_user.login.clone();
        let avatar_url = github_user.avatar_url.clone();
        let id = Uuid::new_v4();

        #[cfg(feature = "postgres")]
        if let Some(db) = state.db.as_ref() {
            match db
                .create_account_github(
                    &username,
                    github_user.email.as_deref(),
                    github_user.id,
                    Some(&username),
                )
                .await
            {
                Ok(db_id) => {
                    let record = crate::state::AccountRecord {
                        id: db_id,
                        username: username.clone(),
                        email: github_user.email,
                        password_hash: String::new(),
                        is_guest: false,
                        display_name: Some(username.clone()),
                        avatar_id: "tower_01".to_string(),
                        auth_provider: "github".to_string(),
                        github_id: Some(github_user.id),
                    };
                    state.accounts.write().await.insert(username.clone(), record);

                    let access_token = auth::generate_access_token(
                        &state.auth_config,
                        db_id,
                        &username,
                        false,
                    )
                    .unwrap_or_default();
                    let refresh_token =
                        auth::generate_refresh_token(&state.auth_config, db_id, &username)
                            .unwrap_or_default();

                    return (
                        StatusCode::CREATED,
                        Json(serde_json::json!({
                            "player_id": db_id,
                            "username": username,
                            "avatar_url": avatar_url,
                            "access_token": access_token,
                            "refresh_token": refresh_token,
                            "is_new": true,
                        })),
                    );
                }
                Err(e) => {
                    return (
                        StatusCode::CONFLICT,
                        Json(serde_json::json!({ "error": format!("Account creation failed: {e}") })),
                    );
                }
            }
        }

        // In-memory fallback
        let record = crate::state::AccountRecord {
            id,
            username: username.clone(),
            email: github_user.email,
            password_hash: String::new(),
            is_guest: false,
            display_name: Some(username.clone()),
            avatar_id: "tower_01".to_string(),
            auth_provider: "github".to_string(),
            github_id: Some(github_user.id),
        };
        state.accounts.write().await.insert(username.clone(), record);

        let access_token =
            auth::generate_access_token(&state.auth_config, id, &username, false)
                .unwrap_or_default();
        let refresh_token =
            auth::generate_refresh_token(&state.auth_config, id, &username)
                .unwrap_or_default();

        (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "player_id": id,
                "username": username,
                "avatar_url": avatar_url,
                "access_token": access_token,
                "refresh_token": refresh_token,
                "is_new": true,
            })),
        )
    }
}

// ── Profile Endpoints ─────────────────────────────────────────────────────

const AVATAR_LIST: &[&str] = &[
    "tower_01", "tower_02", "satellite_01", "satellite_02",
    "antenna_01", "antenna_02", "router_01", "router_02",
    "cable_01", "cable_02", "server_01", "server_02",
    "phone_01", "phone_02", "modem_01", "modem_02",
    "dish_01", "dish_02", "fiber_01", "fiber_02",
    "switch_01", "switch_02", "relay_01", "relay_02",
    "headset_01", "headset_02", "radio_01", "radio_02",
    "globe_01", "globe_02",
];

async fn list_avatars() -> impl IntoResponse {
    Json(serde_json::json!({ "avatars": AVATAR_LIST }))
}

async fn get_own_profile(
    AuthClaims(account_id): AuthClaims,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.get_profile(account_id).await {
            Ok(Some(p)) => {
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "id": p.id,
                        "username": p.username,
                        "display_name": p.display_name,
                        "avatar_id": p.avatar_id.unwrap_or_else(|| "tower_01".to_string()),
                        "auth_provider": p.auth_provider.unwrap_or_else(|| "local".to_string()),
                        "created_at": p.created_at.to_rfc3339(),
                    })),
                );
            }
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "Profile not found" })),
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

    // In-memory fallback: look up by ID
    let accounts = state.accounts.read().await;
    for record in accounts.values() {
        if record.id == account_id {
            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "id": record.id,
                    "username": record.username,
                    "display_name": record.display_name,
                    "avatar_id": record.avatar_id,
                    "auth_provider": record.auth_provider,
                    "created_at": null,
                })),
            );
        }
    }

    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "Profile not found" })),
    )
}

#[derive(Deserialize)]
struct UpdateProfileRequest {
    display_name: Option<String>,
    avatar_id: Option<String>,
}

async fn update_profile(
    AuthClaims(account_id): AuthClaims,
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    // Validate avatar_id
    if let Some(ref avatar) = body.avatar_id {
        if !AVATAR_LIST.contains(&avatar.as_str()) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid avatar ID" })),
            );
        }
    }

    // Validate display_name length
    if let Some(ref name) = body.display_name {
        if name.len() > 64 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Display name must be 64 characters or less" })),
            );
        }
    }

    let avatar = body.avatar_id.as_deref().unwrap_or("tower_01");

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.update_profile(account_id, body.display_name.as_deref(), avatar).await {
            Ok(()) => {
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({ "updated": true })),
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

    // In-memory fallback
    let mut accounts = state.accounts.write().await;
    for record in accounts.values_mut() {
        if record.id == account_id {
            if let Some(ref name) = body.display_name {
                record.display_name = Some(name.clone());
            }
            record.avatar_id = avatar.to_string();
            return (
                StatusCode::OK,
                Json(serde_json::json!({ "updated": true })),
            );
        }
    }

    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "Account not found" })),
    )
}

async fn get_player_profile(
    _: AuthClaims,
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<Uuid>,
) -> impl IntoResponse {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.get_profile(player_id).await {
            Ok(Some(p)) => {
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "id": p.id,
                        "username": p.username,
                        "display_name": p.display_name,
                        "avatar_id": p.avatar_id.unwrap_or_else(|| "tower_01".to_string()),
                    })),
                );
            }
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "Player not found" })),
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

    // In-memory fallback
    let accounts = state.accounts.read().await;
    for record in accounts.values() {
        if record.id == player_id {
            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "id": record.id,
                    "username": record.username,
                    "display_name": record.display_name,
                    "avatar_id": record.avatar_id,
                })),
            );
        }
    }

    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "Player not found" })),
    )
}

async fn delete_account(
    AuthClaims(account_id): AuthClaims,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.soft_delete_account(account_id).await {
            Ok(()) => {
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({ "deleted": true })),
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

    // In-memory: remove from accounts
    let mut accounts = state.accounts.write().await;
    let key = accounts
        .iter()
        .find(|(_, v)| v.id == account_id)
        .map(|(k, _)| k.clone());
    if let Some(k) = key {
        accounts.remove(&k);
        (
            StatusCode::OK,
            Json(serde_json::json!({ "deleted": true })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Account not found" })),
        )
    }
}

// ── Password Reset ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct PasswordResetRequest {
    username: String,
}

async fn request_password_reset(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PasswordResetRequest>,
) -> impl IntoResponse {
    // Always return success to prevent account enumeration
    let success_msg = serde_json::json!({
        "message": "If the account exists, a password reset has been queued."
    });

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        if let Ok(Some(row)) = db.get_account_by_username(&body.username).await {
            // If CF Worker URL is configured and user has email, create a token and send email
            if let (Some(ref cf_url), Some(ref email)) =
                (&state.cf_reset_worker_url, &row.email)
            {
                // Generate a cryptographic token
                let raw_token = crate::state::generate_invite_code()
                    + &crate::state::generate_invite_code(); // 16 chars
                let token_hash = {
                    use sha2::{Sha256, Digest};
                    let mut hasher = Sha256::new();
                    hasher.update(raw_token.as_bytes());
                    format!("{:x}", hasher.finalize())
                };
                let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
                let _ = db.create_reset_token(row.id, &token_hash, expires_at).await;

                // Send via CF Worker (fire and forget)
                let cf_url = cf_url.clone();
                let email = email.clone();
                let token = raw_token;
                tokio::spawn(async move {
                    let client = reqwest::Client::new();
                    let _ = client
                        .post(&cf_url)
                        .json(&serde_json::json!({
                            "email": email,
                            "token": token,
                        }))
                        .send()
                        .await;
                });
            } else {
                // Fallback: queue a manual reset request for admin
                let _ = db.create_reset_request(row.id, &body.username).await;
            }
        }
    }

    (StatusCode::OK, Json(success_msg))
}

#[derive(Deserialize)]
struct PasswordResetConfirm {
    token: String,
    new_password: String,
}

async fn confirm_password_reset(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PasswordResetConfirm>,
) -> impl IntoResponse {
    if body.new_password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Password must be at least 8 characters" })),
        );
    }

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        // Hash the token for lookup (SHA-256, matching create_reset_token)
        let token_hash = {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(body.token.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        match db.validate_reset_token(&token_hash).await {
            Ok(Some(account_id)) => {
                let new_hash = match auth::hash_password(&body.new_password) {
                    Ok(h) => h,
                    Err(_) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({ "error": "Internal error" })),
                        );
                    }
                };

                let _ = db.update_password(account_id, &new_hash).await;
                let _ = db.mark_reset_token_used(&token_hash).await;

                return (
                    StatusCode::OK,
                    Json(serde_json::json!({ "reset": true })),
                );
            }
            Ok(None) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": "Invalid or expired token" })),
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

/// Maximum cloud save size in bytes (50 MB)
const MAX_SAVE_SIZE: usize = 50_000_000;

async fn upload_save(
    AuthClaims(account_id): AuthClaims,
    State(state): State<Arc<AppState>>,
    Json(body): Json<SaveUploadRequest>,
) -> impl IntoResponse {
    // Serialize config to bytes for save data
    let save_data = serde_json::to_vec(&body.config_json).unwrap_or_default();
    if save_data.len() > MAX_SAVE_SIZE {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("Save exceeds maximum size of {} bytes", MAX_SAVE_SIZE) })),
        );
    }
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        // Try R2 first
        #[cfg(feature = "r2")]
        if let Some(ref r2) = state.r2 {
            let r2_key = crate::r2::R2Storage::save_key(account_id, body.slot);
            let size_bytes = save_data.len() as i64;
            match r2.put(&r2_key, &save_data).await {
                Ok(()) => {
                    match db.save_cloud_metadata(
                        account_id, body.slot, &body.name, body.tick,
                        &body.config_json, size_bytes, &r2_key,
                    ).await {
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
                                Json(serde_json::json!({ "error": format!("Save metadata failed: {e}") })),
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("R2 upload failed, falling back to DB: {e}");
                    // Fall through to DB blob path
                }
            }
        }

        // DB blob fallback
        match db
            .save_cloud(
                account_id,
                body.slot,
                &body.name,
                &save_data,
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
                            "id": s.id,
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
    AuthClaims(account_id): AuthClaims,
    State(state): State<Arc<AppState>>,
    Path(slot): Path<i32>,
) -> impl IntoResponse {
    let _ = (&state, slot);
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        // Try R2 first if metadata has an r2_key
        #[cfg(feature = "r2")]
        if let Some(ref r2) = state.r2 {
            if let Ok(Some(meta)) = db.load_cloud_metadata(account_id, slot).await {
                if let Some(ref r2_key) = meta.r2_key {
                    match r2.get(r2_key).await {
                        Ok(Some(data)) => {
                            return (StatusCode::OK, data).into_response();
                        }
                        Ok(None) => {
                            tracing::warn!("R2 key '{}' not found, trying DB blob", r2_key);
                        }
                        Err(e) => {
                            tracing::warn!("R2 GET failed: {e}, trying DB blob");
                        }
                    }
                }
            }
        }

        // DB blob fallback
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
    AuthClaims(account_id): AuthClaims,
    State(state): State<Arc<AppState>>,
    Path(slot): Path<i32>,
) -> impl IntoResponse {
    let _ = (&state, slot);
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        // Delete R2 object if it exists
        #[cfg(feature = "r2")]
        if let Some(ref r2) = state.r2 {
            if let Ok(Some(meta)) = db.load_cloud_metadata(account_id, slot).await {
                if let Some(ref r2_key) = meta.r2_key {
                    if let Err(e) = r2.delete(r2_key).await {
                        tracing::warn!("Failed to delete R2 object {r2_key}: {e}");
                    }
                }
            }
        }

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

// ── Phase 2: World Catalog ─────────────────────────────────────────────────

/// List enabled world templates (public catalog)
async fn list_catalog(
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
async fn get_catalog_template(
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
struct CreateFromTemplateRequest {
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
async fn create_world_from_template(
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
async fn get_world_by_invite(
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

// ── Phase 2: Admin - Templates ─────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateTemplateRequest {
    name: String,
    description: String,
    icon: String,
    config_defaults: serde_json::Value,
    config_bounds: serde_json::Value,
    max_instances: Option<i32>,
    enabled: Option<bool>,
    sort_order: Option<i32>,
}

async fn admin_create_template(
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

async fn admin_list_templates(
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

async fn admin_update_template(
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

async fn admin_delete_template(
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

// ── Phase 2: Admin - Enhanced Bans, Audit, Reset Queue, Metrics ────────────

async fn admin_list_bans(
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

async fn admin_list_reset_queue(
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
struct ResolveResetRequest {
    request_id: Uuid,
}

async fn admin_resolve_reset(
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
async fn admin_metrics(
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

// ── Phase 3: Social System ─────────────────────────────────────────────────

/// List friends with online status
async fn list_friends(
    State(state): State<Arc<AppState>>,
    AuthClaims(player_id): AuthClaims,
) -> impl IntoResponse {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.list_friends(player_id).await {
            Ok(friends) => {
                let online = state.online_players.read().await;
                let result: Vec<serde_json::Value> = friends
                    .into_iter()
                    .map(|f| {
                        let presence = online.get(&f.friend_id);
                        serde_json::json!({
                            "friendship_id": f.id,
                            "id": f.friend_id,
                            "username": f.friend_username,
                            "display_name": f.friend_display_name,
                            "avatar_id": f.friend_avatar_id,
                            "status": f.status,
                            "online": presence.is_some(),
                            "world_id": presence.and_then(|p| p.world_id),
                            "world_name": presence.and_then(|p| p.world_name.clone()),
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
struct FriendRequestBody {
    username: String,
}

/// Send a friend request by username
async fn send_friend_request(
    State(state): State<Arc<AppState>>,
    AuthClaims(player_id): AuthClaims,
    Json(req): Json<FriendRequestBody>,
) -> impl IntoResponse {
    // Look up target by username
    let target = state.get_account(&req.username).await;
    let target = match target {
        Some(t) => t,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "User not found" })),
            );
        }
    };

    if target.id == player_id {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Cannot send friend request to yourself" })),
        );
    }

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.send_friend_request(player_id, target.id).await {
            Ok(request_id) => {
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "status": "sent",
                        "request_id": request_id,
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

    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": "Friends require database" })),
    )
}

/// List incoming and outgoing friend requests
async fn list_friend_requests(
    State(state): State<Arc<AppState>>,
    AuthClaims(player_id): AuthClaims,
) -> impl IntoResponse {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        let incoming = db.list_friend_requests_incoming(player_id).await.unwrap_or_default();
        let outgoing = db.list_friend_requests_outgoing(player_id).await.unwrap_or_default();

        let incoming_json: Vec<serde_json::Value> = incoming
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "from_id": r.from_id,
                    "from_username": r.from_username,
                    "to_id": r.to_id,
                    "to_username": r.to_username,
                    "status": r.status,
                    "created_at": r.created_at.to_rfc3339(),
                })
            })
            .collect();

        let outgoing_json: Vec<serde_json::Value> = outgoing
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "from_id": r.from_id,
                    "from_username": r.from_username,
                    "to_id": r.to_id,
                    "to_username": r.to_username,
                    "status": r.status,
                    "created_at": r.created_at.to_rfc3339(),
                })
            })
            .collect();

        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "incoming": incoming_json,
                "outgoing": outgoing_json,
            })),
        );
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({ "incoming": [], "outgoing": [] })),
    )
}

#[derive(Deserialize)]
struct AcceptRejectRequest {
    request_id: Uuid,
}

/// Accept a friend request
async fn accept_friend_request(
    State(state): State<Arc<AppState>>,
    AuthClaims(_player_id): AuthClaims,
    Json(req): Json<AcceptRejectRequest>,
) -> impl IntoResponse {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.accept_friend_request(req.request_id).await {
            Ok(Some(_)) => {
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({ "status": "accepted" })),
                );
            }
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "Request not found or already handled" })),
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
        Json(serde_json::json!({ "error": "Friends require database" })),
    )
}

/// Reject a friend request
async fn reject_friend_request(
    State(state): State<Arc<AppState>>,
    AuthClaims(_player_id): AuthClaims,
    Json(req): Json<AcceptRejectRequest>,
) -> impl IntoResponse {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.reject_friend_request(req.request_id).await {
            Ok(true) => {
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({ "status": "rejected" })),
                );
            }
            Ok(false) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "Request not found or already handled" })),
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
        Json(serde_json::json!({ "error": "Friends require database" })),
    )
}

/// Remove a friend
async fn remove_friend(
    State(state): State<Arc<AppState>>,
    AuthClaims(player_id): AuthClaims,
    Path(friend_id): Path<Uuid>,
) -> impl IntoResponse {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.remove_friend(player_id, friend_id).await {
            Ok(true) => {
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({ "status": "removed" })),
                );
            }
            Ok(false) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "Friendship not found" })),
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
        Json(serde_json::json!({ "error": "Friends require database" })),
    )
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

/// Search users by username
async fn search_users(
    State(state): State<Arc<AppState>>,
    AuthClaims(_player_id): AuthClaims,
    axum::extract::Query(query): axum::extract::Query<SearchQuery>,
) -> impl IntoResponse {
    if query.q.len() < 2 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Search query must be at least 2 characters" })),
        );
    }

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.search_accounts(&query.q, 20).await {
            Ok(profiles) => {
                let result: Vec<serde_json::Value> = profiles
                    .into_iter()
                    .map(|p| {
                        serde_json::json!({
                            "id": p.id,
                            "username": p.username,
                            "display_name": p.display_name,
                            "avatar_id": p.avatar_id.unwrap_or_else(|| "tower_01".to_string()),
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
struct InviteFriendRequest {
    friend_id: Uuid,
    world_id: Uuid,
}

/// Send a world invite to a friend (via WebSocket broadcast)
async fn invite_friend_to_world(
    State(state): State<Arc<AppState>>,
    AuthClaims(player_id): AuthClaims,
    Json(req): Json<InviteFriendRequest>,
) -> impl IntoResponse {
    // Get the sender's username
    let sender_name = {
        let players = state.players.read().await;
        players
            .get(&player_id)
            .map(|p| p.username.clone())
            .unwrap_or_else(|| "Unknown".to_string())
    };

    // Get the world info
    let world = state.get_world(&req.world_id).await;
    let (world_name, invite_code) = match world {
        Some(ref w) => (w.name.clone(), w.invite_code.clone().unwrap_or_default()),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "World not found" })),
            );
        }
    };

    // Try to deliver the invite via the friend's current world broadcast channel.
    // The WorldInvite message goes to all players in that world, but the frontend
    // only acts on invites matching their player_id (included in from_username).
    let friend_online = state.online_players.read().await.get(&req.friend_id).cloned();
    let delivered = if let Some(presence) = friend_online {
        if let Some(friend_world_id) = presence.world_id {
            // Friend is in a world, send invite via that world's broadcast
            state.broadcast_to_world(
                &friend_world_id,
                gt_common::protocol::ServerMessage::WorldInvite {
                    from_username: sender_name.clone(),
                    world_id: req.world_id,
                    world_name: world_name.clone(),
                    invite_code: invite_code.clone(),
                },
            ).await
        } else {
            false
        }
    } else {
        false
    };

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "invited",
            "world_name": world_name,
            "invite_code": invite_code,
            "delivered": delivered,
        })),
    )
}

/// List recent players from shared worlds
async fn list_recent_players(
    State(state): State<Arc<AppState>>,
    AuthClaims(player_id): AuthClaims,
) -> impl IntoResponse {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.list_recent_players(player_id, 50).await {
            Ok(players) => {
                let online = state.online_players.read().await;
                let result: Vec<serde_json::Value> = players
                    .into_iter()
                    .map(|p| {
                        serde_json::json!({
                            "id": p.other_id,
                            "username": p.username,
                            "display_name": p.display_name,
                            "avatar_id": p.avatar_id,
                            "last_seen": p.last_seen.to_rfc3339(),
                            "online": online.contains_key(&p.other_id),
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

/// List world history for the authenticated player
async fn list_world_history(
    State(state): State<Arc<AppState>>,
    AuthClaims(player_id): AuthClaims,
) -> impl IntoResponse {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.list_world_history(player_id, 20).await {
            Ok(history) => {
                // Check which worlds are still active
                let worlds = state.worlds.read().await;
                let result: Vec<serde_json::Value> = history
                    .into_iter()
                    .map(|h| {
                        let active = worlds.contains_key(&h.world_id);
                        serde_json::json!({
                            "world_id": h.world_id,
                            "world_name": h.world_name,
                            "last_played": h.last_played.to_rfc3339(),
                            "active": active,
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

/// Get per-world leaderboard
async fn get_world_leaderboard(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<Uuid>,
) -> impl IntoResponse {
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.get_top_leaderboard(world_id, 50).await {
            Ok(entries) => {
                let result: Vec<serde_json::Value> = entries
                    .into_iter()
                    .map(|e| {
                        serde_json::json!({
                            "account_id": e.account_id,
                            "corp_name": e.corp_name,
                            "score": e.score,
                            "net_worth": e.net_worth,
                            "tick": e.tick,
                        })
                    })
                    .collect();
                return (StatusCode::OK, Json(serde_json::json!({ "leaderboard": result })));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("Database error: {e}") })),
                );
            }
        }
    }

    let _ = (world_id, &state);
    (StatusCode::OK, Json(serde_json::json!({ "leaderboard": [] })))
}

/// Link an existing local account to a GitHub account
#[derive(Deserialize)]
struct LinkGitHubRequest {
    code: String,
}

async fn link_github_account(
    State(state): State<Arc<AppState>>,
    AuthClaims(player_id): AuthClaims,
    Json(body): Json<LinkGitHubRequest>,
) -> impl IntoResponse {
    #[cfg(not(feature = "oauth"))]
    {
        let _ = (&state, &player_id, &body);
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "OAuth feature not enabled" })),
        );
    }

    #[cfg(feature = "oauth")]
    {
        let oauth_config = match &state.oauth_config {
            Some(c) => c,
            None => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({ "error": "GitHub OAuth not configured" })),
                );
            }
        };

        let github_user = match crate::oauth::github_exchange(oauth_config, &body.code).await {
            Ok(u) => u,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": format!("GitHub auth failed: {e}") })),
                );
            }
        };

        // Check if this GitHub ID is already linked to another account
        if state.get_account_by_github_id(github_user.id).await.is_some() {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({ "error": "This GitHub account is already linked to another user" })),
            );
        }

        // Link the GitHub ID to the authenticated user's account
        #[cfg(feature = "postgres")]
        if let Some(db) = state.db.as_ref() {
            if let Err(e) = db.link_github(player_id, github_user.id).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("Failed to link GitHub: {e}") })),
                );
            }
        }

        // Update in-memory cache
        {
            let mut accounts = state.accounts.write().await;
            for record in accounts.values_mut() {
                if record.id == player_id {
                    record.github_id = Some(github_user.id);
                    break;
                }
            }
        }

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "linked": true,
                "github_login": github_user.login,
            })),
        )
    }
}
