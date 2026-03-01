//! Friends system: requests, friendships, recent players.

use super::Database;
use uuid::Uuid;

// ── Row types ────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
pub struct FriendshipRow {
    pub id: Uuid,
    pub friend_id: Uuid,
    pub friend_username: String,
    pub friend_display_name: Option<String>,
    pub friend_avatar_id: String,
    pub status: String,
}

#[derive(sqlx::FromRow)]
pub struct FriendRequestRow {
    pub id: Uuid,
    pub from_id: Uuid,
    pub from_username: String,
    pub to_id: Uuid,
    pub to_username: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
pub struct RecentPlayerRow {
    pub other_id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_id: String,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

impl Database {
    // ── Friends System ──────────────────────────────────────────────────

    pub async fn send_friend_request(
        &self,
        from_id: Uuid,
        to_id: Uuid,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO friend_requests (id, from_id, to_id) VALUES ($1, $2, $3)
             ON CONFLICT (from_id, to_id) DO NOTHING",
        )
        .bind(id)
        .bind(from_id)
        .bind(to_id)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn accept_friend_request(
        &self,
        request_id: Uuid,
    ) -> Result<Option<(Uuid, Uuid)>, sqlx::Error> {
        // Get the request
        let row: Option<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT from_id, to_id FROM friend_requests WHERE id = $1 AND status = 'pending'",
        )
        .bind(request_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((from_id, to_id)) = row {
            // Update request status
            sqlx::query("UPDATE friend_requests SET status = 'accepted' WHERE id = $1")
                .bind(request_id)
                .execute(&self.pool)
                .await?;

            // Create friendship (ordered: smaller UUID first)
            let (a, b) = if from_id < to_id {
                (from_id, to_id)
            } else {
                (to_id, from_id)
            };
            sqlx::query(
                "INSERT INTO friendships (account_a, account_b, status)
                 VALUES ($1, $2, 'accepted')
                 ON CONFLICT (account_a, account_b) DO UPDATE SET status = 'accepted', updated_at = NOW()",
            )
            .bind(a)
            .bind(b)
            .execute(&self.pool)
            .await?;

            Ok(Some((from_id, to_id)))
        } else {
            Ok(None)
        }
    }

    pub async fn reject_friend_request(&self, request_id: Uuid) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("UPDATE friend_requests SET status = 'rejected' WHERE id = $1 AND status = 'pending'")
                .bind(request_id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn remove_friend(
        &self,
        account_a: Uuid,
        account_b: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let (a, b) = if account_a < account_b {
            (account_a, account_b)
        } else {
            (account_b, account_a)
        };
        let result =
            sqlx::query("DELETE FROM friendships WHERE account_a = $1 AND account_b = $2")
                .bind(a)
                .bind(b)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn list_friends(
        &self,
        account_id: Uuid,
    ) -> Result<Vec<FriendshipRow>, sqlx::Error> {
        sqlx::query_as::<_, FriendshipRow>(
            "SELECT f.id,
                    CASE WHEN f.account_a = $1 THEN f.account_b ELSE f.account_a END AS friend_id,
                    a.username AS friend_username,
                    a.display_name AS friend_display_name,
                    COALESCE(a.avatar_id, 'tower_01') AS friend_avatar_id,
                    f.status
             FROM friendships f
             JOIN accounts a ON a.id = CASE WHEN f.account_a = $1 THEN f.account_b ELSE f.account_a END
             WHERE (f.account_a = $1 OR f.account_b = $1) AND f.status = 'accepted'
             ORDER BY a.username",
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_friend_requests_incoming(
        &self,
        account_id: Uuid,
    ) -> Result<Vec<FriendRequestRow>, sqlx::Error> {
        sqlx::query_as::<_, FriendRequestRow>(
            "SELECT fr.id, fr.from_id, a_from.username AS from_username, fr.to_id, a_to.username AS to_username, fr.status, fr.created_at
             FROM friend_requests fr
             JOIN accounts a_from ON a_from.id = fr.from_id
             JOIN accounts a_to ON a_to.id = fr.to_id
             WHERE fr.to_id = $1 AND fr.status = 'pending'
             ORDER BY fr.created_at DESC",
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_friend_requests_outgoing(
        &self,
        account_id: Uuid,
    ) -> Result<Vec<FriendRequestRow>, sqlx::Error> {
        sqlx::query_as::<_, FriendRequestRow>(
            "SELECT fr.id, fr.from_id, a_from.username AS from_username, fr.to_id, a_to.username AS to_username, fr.status, fr.created_at
             FROM friend_requests fr
             JOIN accounts a_from ON a_from.id = fr.from_id
             JOIN accounts a_to ON a_to.id = fr.to_id
             WHERE fr.from_id = $1 AND fr.status = 'pending'
             ORDER BY fr.created_at DESC",
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await
    }

    // ── Recent Players ──────────────────────────────────────────────────

    pub async fn add_recent_player(
        &self,
        account_id: Uuid,
        other_id: Uuid,
        world_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO recent_players (account_id, other_id, world_id)
             VALUES ($1, $2, $3)
             ON CONFLICT (account_id, other_id, world_id) DO UPDATE SET last_seen = NOW()",
        )
        .bind(account_id)
        .bind(other_id)
        .bind(world_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_recent_players(
        &self,
        account_id: Uuid,
        limit: i64,
    ) -> Result<Vec<RecentPlayerRow>, sqlx::Error> {
        sqlx::query_as::<_, RecentPlayerRow>(
            "SELECT rp.other_id, a.username, a.display_name, COALESCE(a.avatar_id, 'tower_01') AS avatar_id, rp.last_seen
             FROM recent_players rp
             JOIN accounts a ON a.id = rp.other_id
             WHERE rp.account_id = $1
             GROUP BY rp.other_id, a.username, a.display_name, a.avatar_id, rp.last_seen
             ORDER BY rp.last_seen DESC LIMIT $2",
        )
        .bind(account_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }
}
