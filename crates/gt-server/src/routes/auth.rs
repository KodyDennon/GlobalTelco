use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth;
use crate::state::AppState;

use super::AuthClaims;

#[allow(unused_imports)]
use crate::oauth;

// ── Auth Endpoints ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct RegisterRequest {
    username: String,
    password: String,
    email: String,
}

pub(crate) async fn register(
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
pub(crate) struct LoginRequest {
    username: String,
    password: String,
}

pub(crate) async fn login(
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
pub(crate) struct RefreshTokenRequest {
    refresh_token: String,
}

pub(crate) async fn refresh_token(
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

pub(crate) async fn github_auth_url(
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
pub(crate) struct GitHubCallbackQuery {
    code: String,
}

pub(crate) async fn github_callback(
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

// ── Password Reset ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct PasswordResetRequest {
    username: String,
}

pub(crate) async fn request_password_reset(
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
pub(crate) struct PasswordResetConfirm {
    token: String,
    new_password: String,
}

pub(crate) async fn confirm_password_reset(
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

// ── Account Linking ───────────────────────────────────────────────────────

/// Link an existing local account to a GitHub account
#[derive(Deserialize)]
pub(crate) struct LinkGitHubRequest {
    code: String,
}

pub(crate) async fn link_github_account(
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
