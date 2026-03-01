use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

use super::AuthClaims;

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

pub(crate) async fn list_avatars() -> impl IntoResponse {
    Json(serde_json::json!({ "avatars": AVATAR_LIST }))
}

pub(crate) async fn get_own_profile(
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
pub(crate) struct UpdateProfileRequest {
    display_name: Option<String>,
    avatar_id: Option<String>,
}

pub(crate) async fn update_profile(
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

pub(crate) async fn get_player_profile(
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

pub(crate) async fn delete_account(
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
