mod admin;
mod auth;
mod profile;
mod saves;
mod social;
mod worlds;

use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use tower_http::services::ServeDir;
use uuid::Uuid;

use crate::auth as auth_mod;
use crate::state::AppState;

use self::admin::*;
use self::auth::*;
use self::profile::*;
use self::saves::*;
use self::social::*;
use self::worlds::*;

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

// ── Health ─────────────────────────────────────────────────────────────────

async fn health() -> &'static str {
    "ok"
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

// ── Shared Extractors ──────────────────────────────────────────────────────

use axum::extract::State;

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
        let claims = auth_mod::validate_token(&app_state.auth_config, token).map_err(|_| {
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

/// Validate the admin key from the `X-Admin-Key` request header.
/// ADMIN_KEY env var is required -- if unset, all admin endpoints return 503.
/// Uses constant-time comparison to prevent timing attacks.
pub(crate) fn extract_admin_claims(
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
