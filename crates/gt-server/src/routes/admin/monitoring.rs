use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::state::AppState;

use super::super::extract_admin_claims;

// ── Audit Log ─────────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub(crate) struct AuditLogQuery {
    #[serde(default = "default_audit_limit")]
    limit: Option<i64>,
    #[serde(default)]
    offset: Option<i64>,
    #[serde(default)]
    actor: Option<String>,
}

fn default_audit_limit() -> Option<i64> {
    Some(100)
}

pub(crate) async fn admin_audit_log(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<AuditLogQuery>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    // Use DB-backed paginated audit log when available
    #[cfg(feature = "postgres")]
    if let Some(db) = state.db.as_ref() {
        match db.query_audit_log(limit, offset, params.actor.as_deref()).await {
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

    // Fallback to in-memory audit log (apply pagination)
    let log = state.get_audit_log().await;
    let total = log.len();
    let start = (offset as usize).min(total);
    let end = (start + limit as usize).min(total);
    let page = &log[start..end];
    (StatusCode::OK, Json(serde_json::json!({ "audit_log": page, "total": total })))
}

// ── Admin Health ──────────────────────────────────────────────────────

pub(crate) async fn admin_health(
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

// ── Metrics ───────────────────────────────────────────────────────────

/// Real-time server and world metrics
pub(crate) async fn admin_metrics(
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
        let tick_history: Vec<u64> = instance.tick_history.read().await.iter().copied().collect();
        let system_times: indexmap::IndexMap<String, u64> = w.system_times.clone();
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
            "max_tick_us": instance.max_tick_us.load(std::sync::atomic::Ordering::Relaxed),
            "p99_tick_us": instance.p99_tick_us.load(std::sync::atomic::Ordering::Relaxed),
            "tick_history": tick_history,
            "system_times": system_times,
            "entity_count": w.corporations.len() + w.infra_nodes.len() + w.infra_edges.len(),
            "broadcast_subscribers": instance.broadcast_tx.receiver_count(),
        }));
    }
    let world_count = world_metrics.len();
    drop(worlds);

    let online_count = state.online_players.read().await.len();
    let memory_estimate_bytes = state.memory_usage_estimate().await;
    let memory_mb = memory_estimate_bytes as f64 / 1_048_576.0;
    let ws_msg_per_sec = state.ws_messages_per_sec().await;

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "worlds": world_metrics,
            "server": {
                "uptime_secs": state.uptime_secs(),
                "connected_players": online_count,
                "world_count": world_count,
                "memory_estimate_bytes": memory_estimate_bytes,
                "memory_mb": memory_mb,
                "ws_messages_per_sec": ws_msg_per_sec,
            },
        })),
    )
}

// ── Server Configuration ──────────────────────────────────────────────

pub(crate) async fn admin_server_config(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if let Err(e) = extract_admin_claims(&headers) {
        return e;
    }

    let env_vars = serde_json::json!({
        "ADMIN_KEY": std::env::var("ADMIN_KEY").is_ok(),
        "DATABASE_URL": std::env::var("DATABASE_URL").is_ok(),
        "JWT_SECRET": std::env::var("GT_JWT_SECRET").is_ok(),
        "GITHUB_CLIENT_ID": std::env::var("GITHUB_CLIENT_ID").is_ok(),
        "GITHUB_CLIENT_SECRET": std::env::var("GITHUB_CLIENT_SECRET").is_ok(),
        "TILE_DIR": std::env::var("TILE_DIR").is_ok(),
        "CORS_ORIGIN": std::env::var("CORS_ORIGIN").is_ok(),
        "CORS_ORIGINS": std::env::var("CORS_ORIGINS").is_ok(),
        "R2_ACCOUNT_ID": std::env::var("R2_ACCOUNT_ID").is_ok(),
        "R2_ACCESS_KEY_ID": std::env::var("R2_ACCESS_KEY_ID").is_ok(),
        "R2_SECRET_ACCESS_KEY": std::env::var("R2_SECRET_ACCESS_KEY").is_ok(),
        "R2_BUCKET_NAME": std::env::var("R2_BUCKET_NAME").is_ok(),
    });

    let has_postgres = state.db.is_some();
    let has_r2;
    #[cfg(feature = "r2")]
    {
        has_r2 = state.r2.is_some();
    }
    #[cfg(not(feature = "r2"))]
    {
        has_r2 = false;
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "env_vars": env_vars,
            "database": {
                "connected": has_postgres,
                "pool_size": state.db.as_ref().map(|db| db.pool_size()).unwrap_or(0),
            },
            "features": {
                "postgres": has_postgres,
                "r2": has_r2,
            },
        })),
    )
}
