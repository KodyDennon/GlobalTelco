//! Account CRUD operations.

use super::{AccountRow, Database, ProfileRow};
use uuid::Uuid;

impl Database {
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
}
