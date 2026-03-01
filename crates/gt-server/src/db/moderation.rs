//! Bans and audit log operations.

use super::Database;
use uuid::Uuid;

// ── Row types ────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
pub struct BanRow {
    pub id: Uuid,
    pub account_id: Uuid,
    pub username: String,
    pub world_id: Option<Uuid>,
    pub reason: String,
    pub banned_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(sqlx::FromRow)]
pub struct AuditLogRow {
    pub id: i64,
    pub actor: String,
    pub action: String,
    pub target: Option<String>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Database {
    // ── Bans ────────────────────────────────────────────────────────────

    pub async fn create_ban(
        &self,
        account_id: Uuid,
        world_id: Option<Uuid>,
        reason: &str,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO bans (account_id, world_id, reason, expires_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (account_id, world_id) DO UPDATE SET reason = EXCLUDED.reason, expires_at = EXCLUDED.expires_at, banned_at = NOW()",
        )
        .bind(account_id)
        .bind(world_id)
        .bind(reason)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn remove_ban(
        &self,
        account_id: Uuid,
        world_id: Option<Uuid>,
    ) -> Result<bool, sqlx::Error> {
        let result = if let Some(wid) = world_id {
            sqlx::query("DELETE FROM bans WHERE account_id = $1 AND world_id = $2")
                .bind(account_id)
                .bind(wid)
                .execute(&self.pool)
                .await?
        } else {
            sqlx::query("DELETE FROM bans WHERE account_id = $1 AND world_id IS NULL")
                .bind(account_id)
                .execute(&self.pool)
                .await?
        };
        Ok(result.rows_affected() > 0)
    }

    pub async fn list_bans(&self) -> Result<Vec<BanRow>, sqlx::Error> {
        sqlx::query_as::<_, BanRow>(
            "SELECT b.id, b.account_id, a.username, b.world_id, b.reason, b.banned_at, b.expires_at
             FROM bans b JOIN accounts a ON a.id = b.account_id
             ORDER BY b.banned_at DESC",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn is_banned(
        &self,
        account_id: Uuid,
        world_id: Option<Uuid>,
    ) -> Result<bool, sqlx::Error> {
        // Check global bans + world-specific bans, respecting expiry
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM bans
             WHERE account_id = $1
             AND (world_id IS NULL OR world_id = $2)
             AND (expires_at IS NULL OR expires_at > NOW())",
        )
        .bind(account_id)
        .bind(world_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0 > 0)
    }

    // ── Audit Log ───────────────────────────────────────────────────────

    pub async fn insert_audit_log(
        &self,
        actor: &str,
        action: &str,
        target: Option<&str>,
        details: Option<&serde_json::Value>,
        ip_address: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO audit_log (actor, action, target, details, ip_address)
             VALUES ($1, $2, $3, $4, $5::inet)",
        )
        .bind(actor)
        .bind(action)
        .bind(target)
        .bind(details)
        .bind(ip_address)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn query_audit_log(
        &self,
        limit: i64,
        offset: i64,
        actor_filter: Option<&str>,
    ) -> Result<(Vec<AuditLogRow>, i64), sqlx::Error> {
        let total: (i64,) = if let Some(actor) = actor_filter {
            sqlx::query_as("SELECT COUNT(*) FROM audit_log WHERE actor = $1")
                .bind(actor)
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM audit_log")
                .fetch_one(&self.pool)
                .await?
        };

        let rows = if let Some(actor) = actor_filter {
            sqlx::query_as::<_, AuditLogRow>(
                "SELECT id, actor, action, target, details, ip_address, created_at
                 FROM audit_log WHERE actor = $1
                 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            )
            .bind(actor)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, AuditLogRow>(
                "SELECT id, actor, action, target, details, ip_address, created_at
                 FROM audit_log
                 ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        Ok((rows, total.0))
    }
}
