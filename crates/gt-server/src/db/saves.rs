//! Cloud save operations (R2-backed and legacy DB-backed).

use super::Database;
use uuid::Uuid;

// ── Row types ────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
pub struct CloudSaveMetaRow {
    pub r2_key: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct CloudSaveRow {
    pub id: Uuid,
    pub slot: i32,
    pub name: String,
    pub tick: i64,
    pub size_bytes: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Database {
    // ── Cloud Save Metadata (R2-backed) ─────────────────────────────────

    /// Save cloud save metadata only (blob stored in R2).
    pub async fn save_cloud_metadata(
        &self,
        account_id: Uuid,
        slot: i32,
        name: &str,
        tick: i64,
        config_json: &serde_json::Value,
        size_bytes: i64,
        r2_key: &str,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO cloud_saves (id, account_id, slot, name, tick, config_json, size_bytes, r2_key)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (account_id, slot) DO UPDATE SET
                name = EXCLUDED.name,
                tick = EXCLUDED.tick,
                config_json = EXCLUDED.config_json,
                size_bytes = EXCLUDED.size_bytes,
                r2_key = EXCLUDED.r2_key,
                save_data = NULL,
                created_at = NOW()",
        )
        .bind(id)
        .bind(account_id)
        .bind(slot)
        .bind(name)
        .bind(tick)
        .bind(config_json)
        .bind(size_bytes)
        .bind(r2_key)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    /// Load cloud save metadata (r2_key) for a specific slot.
    pub async fn load_cloud_metadata(
        &self,
        account_id: Uuid,
        slot: i32,
    ) -> Result<Option<CloudSaveMetaRow>, sqlx::Error> {
        sqlx::query_as::<_, CloudSaveMetaRow>(
            "SELECT r2_key FROM cloud_saves WHERE account_id = $1 AND slot = $2",
        )
        .bind(account_id)
        .bind(slot)
        .fetch_optional(&self.pool)
        .await
    }

    // ── Cloud Saves ───────────────────────────────────────────────────────

    pub async fn save_cloud(
        &self,
        account_id: Uuid,
        slot: i32,
        name: &str,
        save_data: &[u8],
        tick: i64,
        config_json: &serde_json::Value,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        let size_bytes = save_data.len() as i64;
        sqlx::query(
            "INSERT INTO cloud_saves (id, account_id, slot, name, save_data, tick, config_json, size_bytes)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (account_id, slot) DO UPDATE SET
                name = EXCLUDED.name,
                save_data = EXCLUDED.save_data,
                tick = EXCLUDED.tick,
                config_json = EXCLUDED.config_json,
                size_bytes = EXCLUDED.size_bytes,
                created_at = NOW()",
        )
        .bind(id)
        .bind(account_id)
        .bind(slot)
        .bind(name)
        .bind(save_data)
        .bind(tick)
        .bind(config_json)
        .bind(size_bytes)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn list_cloud_saves(
        &self,
        account_id: Uuid,
    ) -> Result<Vec<CloudSaveRow>, sqlx::Error> {
        sqlx::query_as::<_, CloudSaveRow>(
            "SELECT id, slot, name, tick, size_bytes, created_at
             FROM cloud_saves WHERE account_id = $1 ORDER BY slot",
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn load_cloud_save(
        &self,
        account_id: Uuid,
        slot: i32,
    ) -> Result<Option<Vec<u8>>, sqlx::Error> {
        let row: Option<(Vec<u8>,)> = sqlx::query_as(
            "SELECT save_data FROM cloud_saves WHERE account_id = $1 AND slot = $2",
        )
        .bind(account_id)
        .bind(slot)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(data,)| data))
    }

    pub async fn delete_cloud_save(
        &self,
        account_id: Uuid,
        slot: i32,
    ) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM cloud_saves WHERE account_id = $1 AND slot = $2")
                .bind(account_id)
                .bind(slot)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }
}
