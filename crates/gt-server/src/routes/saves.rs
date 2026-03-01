use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::state::AppState;

use super::AuthClaims;

// ── Cloud Saves ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct SaveUploadRequest {
    slot: i32,
    name: String,
    tick: i64,
    #[serde(default)]
    config_json: serde_json::Value,
}

/// Maximum cloud save size in bytes (50 MB)
const MAX_SAVE_SIZE: usize = 50_000_000;

pub(crate) async fn upload_save(
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

pub(crate) async fn list_saves(
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

pub(crate) async fn download_save(
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

pub(crate) async fn delete_save(
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
