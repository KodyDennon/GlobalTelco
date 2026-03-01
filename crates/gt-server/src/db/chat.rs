//! Chat message persistence.

use super::Database;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub struct ChatMessageRow {
    pub id: i64,
    pub world_id: Uuid,
    pub account_id: Uuid,
    pub username: String,
    pub message: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Database {
    pub async fn insert_chat_message(
        &self,
        world_id: Uuid,
        account_id: Uuid,
        username: &str,
        message: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO chat_messages (world_id, account_id, username, message)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(world_id)
        .bind(account_id)
        .bind(username)
        .bind(message)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_chat_messages(
        &self,
        world_id: Uuid,
        limit: i64,
        before: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<ChatMessageRow>, sqlx::Error> {
        if let Some(before_dt) = before {
            sqlx::query_as::<_, ChatMessageRow>(
                "SELECT id, world_id, account_id, username, message, created_at
                 FROM chat_messages
                 WHERE world_id = $1 AND created_at < $2
                 ORDER BY created_at DESC
                 LIMIT $3",
            )
            .bind(world_id)
            .bind(before_dt)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, ChatMessageRow>(
                "SELECT id, world_id, account_id, username, message, created_at
                 FROM chat_messages
                 WHERE world_id = $1
                 ORDER BY created_at DESC
                 LIMIT $2",
            )
            .bind(world_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        }
    }
}
