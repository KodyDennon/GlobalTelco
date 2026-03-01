use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

use super::AuthClaims;

// ── Social System ─────────────────────────────────────────────────────────

/// List friends with online status
pub(crate) async fn list_friends(
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
pub(crate) struct FriendRequestBody {
    username: String,
}

/// Send a friend request by username
pub(crate) async fn send_friend_request(
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
pub(crate) async fn list_friend_requests(
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
pub(crate) struct AcceptRejectRequest {
    request_id: Uuid,
}

/// Accept a friend request
pub(crate) async fn accept_friend_request(
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
pub(crate) async fn reject_friend_request(
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
pub(crate) async fn remove_friend(
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
pub(crate) struct SearchQuery {
    q: String,
}

/// Search users by username
pub(crate) async fn search_users(
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
pub(crate) struct InviteFriendRequest {
    friend_id: Uuid,
    world_id: Uuid,
}

/// Send a world invite to a friend (via WebSocket broadcast)
pub(crate) async fn invite_friend_to_world(
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
pub(crate) async fn list_recent_players(
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
pub(crate) async fn list_world_history(
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
pub(crate) async fn get_world_leaderboard(
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
