use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use gt_common::types::EntityId;

use crate::auth;
use crate::state::AppState;

use super::super::extract_admin_claims;

// ── Admin Endpoints ───────────────────────────────────────────────────────

pub(crate) async fn admin_list_players(
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
pub(crate) struct KickRequest {
    player_id: Uuid,
}

pub(crate) async fn admin_kick_player(
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

// ── Ban / Unban Endpoints ──────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct BanRequest {
    account_id: Uuid,
    reason: String,
    #[serde(default)]
    world_id: Option<Uuid>,
    #[serde(default)]
    expires_at: Option<String>,
}

pub(crate) async fn admin_ban_player(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<BanRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    // Parse optional expiry
    let expires_at_dt = body.expires_at.as_deref().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&chrono::Utc))
    });

    if let Some(wid) = body.world_id {
        // World-specific ban
        let world = match state.get_world(&wid).await {
            Some(w) => w,
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "World not found" })),
                );
            }
        };
        world.banned_players.write().await.insert(body.account_id);

        #[cfg(feature = "postgres")]
        if let Some(db) = state.db.as_ref() {
            let _ = db.create_ban(body.account_id, Some(wid), &body.reason, expires_at_dt).await;
            let _ = db
                .insert_audit_log(
                    "admin",
                    "ban_player",
                    Some(&body.account_id.to_string()),
                    Some(&serde_json::json!({ "world_id": wid, "reason": body.reason })),
                    None,
                )
                .await;
        }

        // Also kick them if currently connected
        let kicked = state.kick_player(&body.account_id).await;
        world.remove_player(&body.account_id).await;

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "banned": true,
                "account_id": body.account_id,
                "world_id": wid,
                "also_kicked": kicked,
            })),
        )
    } else {
        // Global ban — add to all worlds
        let worlds = state.worlds.read().await;
        for instance in worlds.values() {
            instance.banned_players.write().await.insert(body.account_id);
        }
        drop(worlds);

        #[cfg(feature = "postgres")]
        if let Some(db) = state.db.as_ref() {
            let _ = db.create_ban(body.account_id, None, &body.reason, expires_at_dt).await;
            let _ = db
                .insert_audit_log(
                    "admin",
                    "ban_player_global",
                    Some(&body.account_id.to_string()),
                    Some(&serde_json::json!({ "reason": body.reason })),
                    None,
                )
                .await;
        }

        let kicked = state.kick_player(&body.account_id).await;

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "banned": true,
                "account_id": body.account_id,
                "world_id": null,
                "also_kicked": kicked,
            })),
        )
    }
}

#[derive(Deserialize)]
pub(crate) struct UnbanRequest {
    account_id: Uuid,
    #[serde(default)]
    world_id: Option<Uuid>,
}

pub(crate) async fn admin_unban_player(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<UnbanRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    if let Some(wid) = body.world_id {
        let world = match state.get_world(&wid).await {
            Some(w) => w,
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "World not found" })),
                );
            }
        };

        let was_banned = world.banned_players.write().await.remove(&body.account_id);

        #[cfg(feature = "postgres")]
        if let Some(db) = state.db.as_ref() {
            let _ = db.remove_ban(body.account_id, Some(wid)).await;
            let _ = db
                .insert_audit_log(
                    "admin",
                    "unban_player",
                    Some(&body.account_id.to_string()),
                    Some(&serde_json::json!({ "world_id": wid })),
                    None,
                )
                .await;
        }

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "unbanned": was_banned,
                "account_id": body.account_id,
                "world_id": wid,
            })),
        )
    } else {
        // Global unban — remove from all worlds
        let worlds = state.worlds.read().await;
        let mut was_banned = false;
        for instance in worlds.values() {
            if instance.banned_players.write().await.remove(&body.account_id) {
                was_banned = true;
            }
        }
        drop(worlds);

        #[cfg(feature = "postgres")]
        if let Some(db) = state.db.as_ref() {
            let _ = db.remove_ban(body.account_id, None).await;
            let _ = db
                .insert_audit_log(
                    "admin",
                    "unban_player_global",
                    Some(&body.account_id.to_string()),
                    None,
                    None,
                )
                .await;
        }

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "unbanned": was_banned,
                "account_id": body.account_id,
                "world_id": null,
            })),
        )
    }
}

// ── Enhanced Bans ─────────────────────────────────────────────────────

pub(crate) async fn admin_list_bans(
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

// ── Reset Queue ───────────────────────────────────────────────────────

pub(crate) async fn admin_list_reset_queue(
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
pub(crate) struct ResolveResetRequest {
    request_id: Uuid,
}

pub(crate) async fn admin_resolve_reset(
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

        let _ = db
            .insert_audit_log(
                "admin",
                "resolve_reset",
                Some(&req.request_id.to_string()),
                Some(&serde_json::json!({ "account_id": pending.account_id })),
                None,
            )
            .await;

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

// ── List Accounts ─────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub(crate) struct ListAccountsQuery {
    #[serde(default)]
    search: Option<String>,
    #[serde(default = "default_page")]
    page: Option<i64>,
    #[serde(default = "default_per_page")]
    per_page: Option<i64>,
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    order: Option<String>,
}

fn default_page() -> Option<i64> {
    Some(1)
}

fn default_per_page() -> Option<i64> {
    Some(50)
}

pub(crate) async fn admin_list_accounts(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<ListAccountsQuery>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;
    let sort = params.sort.as_deref().unwrap_or("created_at");
    let order = params.order.as_deref().unwrap_or("desc");

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.list_accounts(params.search.as_deref(), per_page, offset, sort, order).await {
            Ok((accounts, total)) => {
                let result: Vec<serde_json::Value> = accounts
                    .into_iter()
                    .map(|a| {
                        serde_json::json!({
                            "id": a.id,
                            "username": a.username,
                            "email": a.email,
                            "display_name": a.display_name,
                            "avatar_id": a.avatar_id,
                            "auth_provider": a.auth_provider,
                            "is_guest": a.is_guest,
                            "created_at": a.created_at,
                            "last_login": a.last_login,
                            "deleted_at": a.deleted_at,
                        })
                    })
                    .collect();
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "accounts": result,
                        "total": total,
                        "page": page,
                        "per_page": per_page,
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

    // Fallback: return in-memory accounts
    let accounts = state.accounts.read().await;
    let list: Vec<serde_json::Value> = accounts
        .values()
        .map(|a| {
            serde_json::json!({
                "id": a.id,
                "username": a.username,
                "email": a.email,
                "display_name": a.display_name,
                "avatar_id": a.avatar_id,
                "auth_provider": a.auth_provider,
                "is_guest": a.is_guest,
            })
        })
        .collect();
    let total = list.len();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "accounts": list,
            "total": total,
            "page": page,
            "per_page": per_page,
        })),
    )
}

// ── List Connections ──────────────────────────────────────────────────

pub(crate) async fn admin_list_connections(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let players = state.players.read().await;
    let online = state.online_players.read().await;
    let worlds = state.worlds.read().await;

    let connections: Vec<serde_json::Value> = players
        .values()
        .map(|p| {
            let presence = online.get(&p.id);
            let world_name = p.world_id.and_then(|wid| {
                worlds.get(&wid).map(|w| w.name.clone())
            });
            serde_json::json!({
                "id": p.id,
                "username": p.username,
                "world_id": p.world_id,
                "world_name": world_name,
                "corp_id": p.corp_id,
                "is_guest": p.is_guest,
                "is_spectator": p.is_spectator,
                "connected_at": presence.map(|pr| pr.connected_at),
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(serde_json::json!({ "connections": connections })),
    )
}

// ── Assign Player ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct AssignPlayerRequest {
    player_id: Uuid,
    corp_id: EntityId,
}

pub(crate) async fn admin_assign_player(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<Uuid>,
    Json(body): Json<AssignPlayerRequest>,
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

    // Update the world's players map
    world.players.write().await.insert(body.player_id, body.corp_id);

    // Update the connected player's corp_id if they're online
    if let Some(player) = state.players.write().await.get_mut(&body.player_id) {
        player.corp_id = Some(body.corp_id);
    }

    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        let _ = db
            .insert_audit_log(
                "admin",
                "assign_player",
                Some(&body.player_id.to_string()),
                Some(&serde_json::json!({ "world_id": world_id, "corp_id": body.corp_id })),
                None,
            )
            .await;
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "assigned": true,
            "player_id": body.player_id,
            "corp_id": body.corp_id,
            "world_id": world_id,
        })),
    )
}

// ── Toggle Spectator ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct SpectatorRequest {
    player_id: Uuid,
    spectator: bool,
}

pub(crate) async fn admin_toggle_spectator(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<Uuid>,
    Json(body): Json<SpectatorRequest>,
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

    let mut players = state.players.write().await;
    if let Some(player) = players.get_mut(&body.player_id) {
        player.is_spectator = body.spectator;
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "player_id": body.player_id,
                "spectator": body.spectator,
                "world_id": world_id,
            })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Player not found" })),
        )
    }
}
