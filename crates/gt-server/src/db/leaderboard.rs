//! Leaderboard rankings and player session tracking.

use super::Database;
use uuid::Uuid;

// ── Row types ────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
pub struct LeaderboardRow {
    pub account_id: Uuid,
    pub corp_name: String,
    pub score: i64,
    pub net_worth: i64,
    pub tick: i64,
}

impl Database {
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
