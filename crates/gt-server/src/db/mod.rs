//! Database persistence layer for GlobalTelco server.
//!
//! When the `postgres` feature is enabled, provides PostgreSQL-backed storage
//! for accounts, worlds, snapshots, cloud saves, events, and leaderboard.
//! Falls back to in-memory storage when not enabled.

#[cfg(feature = "postgres")]
mod accounts;
#[cfg(feature = "postgres")]
mod auth;
#[cfg(feature = "postgres")]
mod chat;
#[cfg(feature = "postgres")]
mod leaderboard;
#[cfg(feature = "postgres")]
mod moderation;
#[cfg(feature = "postgres")]
mod saves;
#[cfg(feature = "postgres")]
mod social;
#[cfg(feature = "postgres")]
mod worlds;

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
        let migration1 = include_str!("../../migrations/001_initial_schema.sql");
        sqlx::raw_sql(migration1).execute(&self.pool).await?;
        let migration2 = include_str!("../../migrations/002_multiplayer_overhaul.sql");
        sqlx::raw_sql(migration2).execute(&self.pool).await?;
        let migration3 = include_str!("../../migrations/003_r2_storage.sql");
        sqlx::raw_sql(migration3).execute(&self.pool).await?;
        let migration4 = include_str!("../../migrations/004_chat_messages.sql");
        sqlx::raw_sql(migration4).execute(&self.pool).await?;
        Ok(())
    }
}

// ── Row types used across multiple submodules ────────────────────────────

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

// ── Non-postgres stub ─────────────────────────────────────────────────────

/// Stub when postgres feature is not enabled
#[cfg(not(feature = "postgres"))]
pub struct Database;
