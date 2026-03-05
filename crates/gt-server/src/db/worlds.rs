//! World templates, world CRUD, snapshots, world history, and event log.

use super::Database;
use uuid::Uuid;

// ── Row types ────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
pub struct WorldTemplateRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub config_defaults: serde_json::Value,
    pub config_bounds: serde_json::Value,
    pub max_instances: i32,
    pub enabled: bool,
    pub sort_order: i32,
}

#[derive(sqlx::FromRow)]
pub struct WorldHistoryRow {
    pub world_id: Uuid,
    pub world_name: String,
    pub last_played: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
pub struct WorldRow {
    pub id: Uuid,
    pub name: String,
    pub config_json: serde_json::Value,
    pub current_tick: i64,
    pub speed: String,
    pub max_players: i32,
}

#[derive(sqlx::FromRow)]
pub struct SnapshotRow {
    pub tick: i64,
    pub state_data: Vec<u8>,
}

#[derive(sqlx::FromRow)]
pub struct SnapshotMetaRow {
    pub tick: i64,
    pub r2_key: String,
}

impl Database {
    // ── World Templates ─────────────────────────────────────────────────

    pub async fn create_world_template(
        &self,
        name: &str,
        description: &str,
        icon: &str,
        config_defaults: &serde_json::Value,
        config_bounds: &serde_json::Value,
        max_instances: i32,
        enabled: bool,
        sort_order: i32,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO world_templates (id, name, description, icon, config_defaults, config_bounds, max_instances, enabled, sort_order)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(icon)
        .bind(config_defaults)
        .bind(config_bounds)
        .bind(max_instances)
        .bind(enabled)
        .bind(sort_order)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn list_world_templates(
        &self,
        enabled_only: bool,
    ) -> Result<Vec<WorldTemplateRow>, sqlx::Error> {
        if enabled_only {
            sqlx::query_as::<_, WorldTemplateRow>(
                "SELECT id, name, description, icon, config_defaults, config_bounds, max_instances, enabled, sort_order
                 FROM world_templates WHERE enabled = TRUE ORDER BY sort_order, name",
            )
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, WorldTemplateRow>(
                "SELECT id, name, description, icon, config_defaults, config_bounds, max_instances, enabled, sort_order
                 FROM world_templates ORDER BY sort_order, name",
            )
            .fetch_all(&self.pool)
            .await
        }
    }

    pub async fn get_template(
        &self,
        template_id: Uuid,
    ) -> Result<Option<WorldTemplateRow>, sqlx::Error> {
        sqlx::query_as::<_, WorldTemplateRow>(
            "SELECT id, name, description, icon, config_defaults, config_bounds, max_instances, enabled, sort_order
             FROM world_templates WHERE id = $1",
        )
        .bind(template_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn update_world_template(
        &self,
        id: Uuid,
        name: &str,
        description: &str,
        icon: &str,
        config_defaults: &serde_json::Value,
        config_bounds: &serde_json::Value,
        max_instances: i32,
        enabled: bool,
        sort_order: i32,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE world_templates SET name=$1, description=$2, icon=$3, config_defaults=$4, config_bounds=$5,
             max_instances=$6, enabled=$7, sort_order=$8, updated_at=NOW() WHERE id=$9",
        )
        .bind(name)
        .bind(description)
        .bind(icon)
        .bind(config_defaults)
        .bind(config_bounds)
        .bind(max_instances)
        .bind(enabled)
        .bind(sort_order)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_world_template(&self, template_id: Uuid) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM world_templates WHERE id = $1")
                .bind(template_id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn count_template_instances(&self, template_id: Uuid) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM game_worlds WHERE template_id = $1 AND status = 'active'",
        )
        .bind(template_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    pub async fn set_world_template_id(
        &self,
        world_id: Uuid,
        template_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE game_worlds SET template_id = $1 WHERE id = $2")
            .bind(template_id)
            .bind(world_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_world_creator(
        &self,
        world_id: Uuid,
        account_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE game_worlds SET created_by = $1 WHERE id = $2")
            .bind(account_id)
            .bind(world_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_world_invite_code(
        &self,
        world_id: Uuid,
        code: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE game_worlds SET invite_code = $1 WHERE id = $2")
            .bind(code)
            .bind(world_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_world_by_invite_code(
        &self,
        code: &str,
    ) -> Result<Option<WorldRow>, sqlx::Error> {
        sqlx::query_as::<_, WorldRow>(
            "SELECT id, name, config_json, current_tick, speed, max_players
             FROM game_worlds WHERE invite_code = $1 AND status = 'active'",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
    }

    /// Count how many active worlds a player has created.
    pub async fn count_worlds_by_creator(&self, account_id: Uuid) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM game_worlds WHERE created_by = $1 AND status = 'active'",
        )
        .bind(account_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    // ── World History ───────────────────────────────────────────────────

    pub async fn upsert_world_history(
        &self,
        account_id: Uuid,
        world_id: Uuid,
        world_name: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO world_history (account_id, world_id, world_name)
             VALUES ($1, $2, $3)
             ON CONFLICT (account_id, world_id) DO UPDATE SET last_played = NOW(), world_name = EXCLUDED.world_name",
        )
        .bind(account_id)
        .bind(world_id)
        .bind(world_name)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_world_history(
        &self,
        account_id: Uuid,
        limit: i64,
    ) -> Result<Vec<WorldHistoryRow>, sqlx::Error> {
        sqlx::query_as::<_, WorldHistoryRow>(
            "SELECT world_id, world_name, last_played
             FROM world_history WHERE account_id = $1
             ORDER BY last_played DESC LIMIT $2",
        )
        .bind(account_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    // ── Worlds ────────────────────────────────────────────────────────────

    pub async fn save_world(
        &self,
        world_id: Uuid,
        name: &str,
        config_json: &serde_json::Value,
        current_tick: i64,
        speed: &str,
        max_players: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO game_worlds (id, name, config_json, current_tick, speed, max_players)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (id) DO UPDATE SET
                current_tick = EXCLUDED.current_tick,
                speed = EXCLUDED.speed,
                updated_at = NOW()",
        )
        .bind(world_id)
        .bind(name)
        .bind(config_json)
        .bind(current_tick)
        .bind(speed)
        .bind(max_players)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_active_worlds(&self) -> Result<Vec<WorldRow>, sqlx::Error> {
        sqlx::query_as::<_, WorldRow>(
            "SELECT id, name, config_json, current_tick, speed, max_players
             FROM game_worlds WHERE status = 'active' ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
    }

    // ── Snapshots ─────────────────────────────────────────────────────────

    pub async fn save_snapshot(
        &self,
        world_id: Uuid,
        tick: i64,
        state_data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO world_snapshots (world_id, tick, state_data) VALUES ($1, $2, $3)",
        )
        .bind(world_id)
        .bind(tick)
        .bind(state_data)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn load_latest_snapshot(
        &self,
        world_id: Uuid,
    ) -> Result<Option<SnapshotRow>, sqlx::Error> {
        sqlx::query_as::<_, SnapshotRow>(
            "SELECT tick, state_data FROM world_snapshots
             WHERE world_id = $1 ORDER BY tick DESC LIMIT 1",
        )
        .bind(world_id)
        .fetch_optional(&self.pool)
        .await
    }

    // ── Snapshot Metadata (R2-backed) ──────────────────────────────────────

    /// Save snapshot metadata only (blob stored in R2).
    pub async fn save_snapshot_metadata(
        &self,
        world_id: Uuid,
        tick: i64,
        r2_key: &str,
        size_bytes: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO world_snapshots (world_id, tick, r2_key, size_bytes)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(world_id)
        .bind(tick)
        .bind(r2_key)
        .bind(size_bytes)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Load the latest snapshot metadata (R2 key + tick) for a world.
    pub async fn load_latest_snapshot_metadata(
        &self,
        world_id: Uuid,
    ) -> Result<Option<SnapshotMetaRow>, sqlx::Error> {
        sqlx::query_as::<_, SnapshotMetaRow>(
            "SELECT tick, r2_key FROM world_snapshots
             WHERE world_id = $1 AND r2_key IS NOT NULL
             ORDER BY tick DESC LIMIT 1",
        )
        .bind(world_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Prune old snapshots, keeping only the most recent `keep_count`.
    /// Returns the R2 keys of pruned snapshots (for R2 deletion).
    pub async fn prune_old_snapshots(
        &self,
        world_id: Uuid,
        keep_count: i64,
    ) -> Result<Vec<String>, sqlx::Error> {
        // Get R2 keys of rows that will be deleted
        let old_keys: Vec<(Option<String>,)> = sqlx::query_as(
            "SELECT r2_key FROM world_snapshots
             WHERE world_id = $1
             ORDER BY tick DESC
             OFFSET $2",
        )
        .bind(world_id)
        .bind(keep_count)
        .fetch_all(&self.pool)
        .await?;

        let r2_keys: Vec<String> = old_keys
            .into_iter()
            .filter_map(|(k,)| k)
            .collect();

        // Delete the old rows
        sqlx::query(
            "DELETE FROM world_snapshots
             WHERE world_id = $1
             AND tick NOT IN (
                 SELECT tick FROM world_snapshots
                 WHERE world_id = $1
                 ORDER BY tick DESC
                 LIMIT $2
             )",
        )
        .bind(world_id)
        .bind(keep_count)
        .execute(&self.pool)
        .await?;

        Ok(r2_keys)
    }

    // ── Event Log ─────────────────────────────────────────────────────────

    pub async fn batch_insert_events(
        &self,
        world_id: Uuid,
        events: &[(i64, &str, &serde_json::Value)],
    ) -> Result<(), sqlx::Error> {
        if events.is_empty() {
            return Ok(());
        }

        // Optimized batch insert for PostgreSQL
        let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            "INSERT INTO event_log (world_id, tick, event_type, event_data) "
        );

        query_builder.push_values(events, |mut b, (tick, event_type, event_data)| {
            b.push_bind(world_id)
             .push_bind(*tick)
             .push_bind(*event_type)
             .push_bind(*event_data);
        });

        let query = query_builder.build();
        query.execute(&self.pool).await?;
        
        Ok(())
    }

    /// Prune old events for a world, keeping only the most recent `keep_ticks`.
    pub async fn prune_old_events(
        &self,
        world_id: Uuid,
        keep_ticks: i64,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            \"DELETE FROM event_log
             WHERE world_id = $1
             AND tick < (
                 SELECT MAX(tick) - $2 FROM event_log WHERE world_id = $1
             )\",
        )
        .bind(world_id)
        .bind(keep_ticks)
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected())
    }
}
