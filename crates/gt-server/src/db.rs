//! Database persistence layer for GlobalTelco server.
//!
//! When the `postgres` feature is enabled, provides PostgreSQL-backed storage
//! for accounts, worlds, snapshots, cloud saves, events, and leaderboard.
//! Falls back to in-memory storage when not enabled.

#[cfg(feature = "postgres")]
use sqlx::PgPool;
use uuid::Uuid;

#[cfg(feature = "postgres")]
pub struct Database {
    pool: PgPool,
}

#[cfg(feature = "postgres")]
impl Database {
    pub async fn connect(url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(url).await?;
        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        // Run the SQL migration file
        let migration = include_str!("../migrations/001_initial_schema.sql");
        sqlx::raw_sql(migration).execute(&self.pool).await?;
        Ok(())
    }

    // ── Accounts ──────────────────────────────────────────────────────────

    pub async fn create_account(
        &self,
        username: &str,
        email: Option<&str>,
        password_hash: &str,
        is_guest: bool,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO accounts (id, username, email, password_hash, is_guest)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(is_guest)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn get_account_by_username(
        &self,
        username: &str,
    ) -> Result<Option<AccountRow>, sqlx::Error> {
        sqlx::query_as::<_, AccountRow>(
            "SELECT id, username, email, password_hash, is_guest FROM accounts WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn update_last_login(&self, account_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE accounts SET last_login = NOW() WHERE id = $1")
            .bind(account_id)
            .execute(&self.pool)
            .await?;
        Ok(())
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

    // ── Event Log ─────────────────────────────────────────────────────────

    pub async fn batch_insert_events(
        &self,
        world_id: Uuid,
        events: &[(i64, &str, &serde_json::Value)],
    ) -> Result<(), sqlx::Error> {
        for (tick, event_type, event_data) in events {
            sqlx::query(
                "INSERT INTO event_log (world_id, tick, event_type, event_data)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(world_id)
            .bind(tick)
            .bind(event_type)
            .bind(event_data)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    // ── Leaderboard ───────────────────────────────────────────────────────

    pub async fn update_leaderboard(
        &self,
        account_id: Uuid,
        world_id: Uuid,
        corp_name: &str,
        score: i64,
        net_worth: i64,
        tick: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO leaderboard (account_id, world_id, corp_name, score, net_worth, tick)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (account_id, world_id) DO UPDATE SET
                score = EXCLUDED.score,
                net_worth = EXCLUDED.net_worth,
                tick = EXCLUDED.tick,
                updated_at = NOW()",
        )
        .bind(account_id)
        .bind(world_id)
        .bind(corp_name)
        .bind(score)
        .bind(net_worth)
        .bind(tick)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_top_leaderboard(
        &self,
        world_id: Uuid,
        limit: i64,
    ) -> Result<Vec<LeaderboardRow>, sqlx::Error> {
        sqlx::query_as::<_, LeaderboardRow>(
            "SELECT account_id, corp_name, score, net_worth, tick
             FROM leaderboard WHERE world_id = $1
             ORDER BY score DESC LIMIT $2",
        )
        .bind(world_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    // ── Player Sessions ───────────────────────────────────────────────────

    pub async fn upsert_player_session(
        &self,
        account_id: Uuid,
        world_id: Uuid,
        corp_id: i64,
        is_connected: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO player_sessions (account_id, world_id, corp_id, is_connected)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (account_id, world_id) DO UPDATE SET
                is_connected = EXCLUDED.is_connected,
                last_active = NOW()",
        )
        .bind(account_id)
        .bind(world_id)
        .bind(corp_id)
        .bind(is_connected)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_player_disconnected(
        &self,
        account_id: Uuid,
        world_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE player_sessions SET is_connected = FALSE, is_ai_proxy = TRUE, last_active = NOW()
             WHERE account_id = $1 AND world_id = $2",
        )
        .bind(account_id)
        .bind(world_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_player_connected(
        &self,
        account_id: Uuid,
        world_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE player_sessions SET is_connected = TRUE, is_ai_proxy = FALSE, last_active = NOW()
             WHERE account_id = $1 AND world_id = $2",
        )
        .bind(account_id)
        .bind(world_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

// Row types for sqlx FromRow
#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
pub struct AccountRow {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub is_guest: bool,
}

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
pub struct WorldRow {
    pub id: Uuid,
    pub name: String,
    pub config_json: serde_json::Value,
    pub current_tick: i64,
    pub speed: String,
    pub max_players: i32,
}

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
pub struct SnapshotRow {
    pub tick: i64,
    pub state_data: Vec<u8>,
}

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
pub struct CloudSaveRow {
    pub id: Uuid,
    pub slot: i32,
    pub name: String,
    pub tick: i64,
    pub size_bytes: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
pub struct LeaderboardRow {
    pub account_id: Uuid,
    pub corp_name: String,
    pub score: i64,
    pub net_worth: i64,
    pub tick: i64,
}

// ── Non-postgres stub ─────────────────────────────────────────────────────

/// Stub when postgres feature is not enabled
#[cfg(not(feature = "postgres"))]
pub struct Database;

#[cfg(not(feature = "postgres"))]
impl Database {
    pub fn unavailable() -> Self {
        Database
    }
}
