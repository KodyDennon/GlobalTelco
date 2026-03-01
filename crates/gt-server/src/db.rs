//! Database persistence layer for GlobalTelco server.
//!
//! When the `postgres` feature is enabled, provides PostgreSQL-backed storage
//! for accounts, worlds, snapshots, cloud saves, events, and leaderboard.
//! Falls back to in-memory storage when not enabled.

#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "postgres")]
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
        // Run the SQL migration files
        let migration1 = include_str!("../migrations/001_initial_schema.sql");
        sqlx::raw_sql(migration1).execute(&self.pool).await?;
        let migration2 = include_str!("../migrations/002_multiplayer_overhaul.sql");
        sqlx::raw_sql(migration2).execute(&self.pool).await?;
        let migration3 = include_str!("../migrations/003_r2_storage.sql");
        sqlx::raw_sql(migration3).execute(&self.pool).await?;
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
        let provider = if is_guest { "guest" } else { "local" };
        sqlx::query(
            "INSERT INTO accounts (id, username, email, password_hash, is_guest, auth_provider)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(is_guest)
        .bind(provider)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn get_account_by_username(
        &self,
        username: &str,
    ) -> Result<Option<AccountRow>, sqlx::Error> {
        sqlx::query_as::<_, AccountRow>(
            "SELECT id, username, email, password_hash, is_guest, display_name, avatar_id, auth_provider, github_id
             FROM accounts WHERE username = $1 AND deleted_at IS NULL",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_account_by_github_id(
        &self,
        github_id: i64,
    ) -> Result<Option<AccountRow>, sqlx::Error> {
        sqlx::query_as::<_, AccountRow>(
            "SELECT id, username, email, password_hash, is_guest, display_name, avatar_id, auth_provider, github_id
             FROM accounts WHERE github_id = $1 AND deleted_at IS NULL",
        )
        .bind(github_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create_account_github(
        &self,
        username: &str,
        email: Option<&str>,
        github_id: i64,
        display_name: Option<&str>,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO accounts (id, username, email, password_hash, is_guest, github_id, auth_provider, display_name)
             VALUES ($1, $2, $3, '', FALSE, $4, 'github', $5)",
        )
        .bind(id)
        .bind(username)
        .bind(email)
        .bind(github_id)
        .bind(display_name)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn link_github(
        &self,
        account_id: Uuid,
        github_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE accounts SET github_id = $1 WHERE id = $2")
            .bind(github_id)
            .bind(account_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_profile(
        &self,
        account_id: Uuid,
        display_name: Option<&str>,
        avatar_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE accounts SET display_name = $1, avatar_id = $2, updated_at = NOW() WHERE id = $3",
        )
        .bind(display_name)
        .bind(avatar_id)
        .bind(account_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_profile(&self, account_id: Uuid) -> Result<Option<ProfileRow>, sqlx::Error> {
        sqlx::query_as::<_, ProfileRow>(
            "SELECT id, username, display_name, avatar_id, auth_provider, created_at
             FROM accounts WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn search_accounts(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<ProfileRow>, sqlx::Error> {
        let pattern = format!("%{query}%");
        sqlx::query_as::<_, ProfileRow>(
            "SELECT id, username, display_name, avatar_id, auth_provider, created_at
             FROM accounts WHERE username ILIKE $1 AND deleted_at IS NULL
             ORDER BY username LIMIT $2",
        )
        .bind(pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn soft_delete_account(&self, account_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE accounts SET deleted_at = NOW() WHERE id = $1")
            .bind(account_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_last_login(&self, account_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE accounts SET last_login = NOW() WHERE id = $1")
            .bind(account_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_password(
        &self,
        account_id: Uuid,
        new_hash: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE accounts SET password_hash = $1, updated_at = NOW() WHERE id = $2")
            .bind(new_hash)
            .bind(account_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

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

// ── Row types for sqlx FromRow ───────────────────────────────────────────

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
pub struct AccountRow {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub is_guest: bool,
    pub display_name: Option<String>,
    pub avatar_id: Option<String>,
    pub auth_provider: Option<String>,
    pub github_id: Option<i64>,
}

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
pub struct ProfileRow {
    pub id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_id: Option<String>,
    pub auth_provider: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
pub struct ResetRequestRow {
    pub id: Uuid,
    pub account_id: Uuid,
    pub username: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "postgres")]
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

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
pub struct FriendshipRow {
    pub id: Uuid,
    pub friend_id: Uuid,
    pub friend_username: String,
    pub friend_display_name: Option<String>,
    pub friend_avatar_id: String,
    pub status: String,
}

#[cfg(feature = "postgres")]
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

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
pub struct RecentPlayerRow {
    pub other_id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_id: String,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "postgres")]
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

#[cfg(feature = "postgres")]
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

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
pub struct WorldHistoryRow {
    pub world_id: Uuid,
    pub world_name: String,
    pub last_played: chrono::DateTime<chrono::Utc>,
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
pub struct SnapshotMetaRow {
    pub tick: i64,
    pub r2_key: String,
}

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
pub struct CloudSaveMetaRow {
    pub r2_key: Option<String>,
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
