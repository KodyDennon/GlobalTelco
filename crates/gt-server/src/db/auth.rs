//! Password reset tokens and admin reset request queue.

use super::Database;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub struct ResetRequestRow {
    pub id: Uuid,
    pub account_id: Uuid,
    pub username: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Database {
    // ── Password Reset Tokens ───────────────────────────────────────────

    pub async fn create_reset_token(
        &self,
        account_id: Uuid,
        token_hash: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO password_reset_tokens (account_id, token_hash, expires_at)
             VALUES ($1, $2, $3)",
        )
        .bind(account_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn validate_reset_token(
        &self,
        token_hash: &str,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT account_id FROM password_reset_tokens
             WHERE token_hash = $1 AND used_at IS NULL AND expires_at > NOW()",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn mark_reset_token_used(&self, token_hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE password_reset_tokens SET used_at = NOW() WHERE token_hash = $1")
            .bind(token_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Password Reset Requests (Admin Queue) ───────────────────────────

    pub async fn create_reset_request(
        &self,
        account_id: Uuid,
        username: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO password_reset_requests (account_id, username)
             VALUES ($1, $2)",
        )
        .bind(account_id)
        .bind(username)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_pending_reset_requests(
        &self,
    ) -> Result<Vec<ResetRequestRow>, sqlx::Error> {
        sqlx::query_as::<_, ResetRequestRow>(
            "SELECT id, account_id, username, status, created_at
             FROM password_reset_requests WHERE status = 'pending'
             ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn resolve_reset_request(
        &self,
        request_id: Uuid,
        resolved_by: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE password_reset_requests SET status = 'resolved', resolved_by = $1, resolved_at = NOW()
             WHERE id = $2",
        )
        .bind(resolved_by)
        .bind(request_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
