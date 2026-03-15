use crate::auth::AuthConfig;
use crate::oauth::OAuthConfig;

/// Server configuration, loaded from environment variables
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub auth: AuthConfig,
    pub database_url: Option<String>,
    pub default_world_name: String,
    pub default_max_players: u32,
    /// Directory containing satellite tile files in `{z}/{y}/{x}.jpg` format.
    /// When set, the server serves tiles at `GET /tiles/{z}/{y}/{x}`.
    pub tile_dir: Option<String>,
    /// GitHub OAuth configuration (optional)
    pub oauth: Option<OAuthConfig>,
    /// Cloudflare Worker URL for sending password reset emails (optional)
    pub cf_reset_worker_url: Option<String>,
    /// Cloudflare R2 configuration (optional — falls back to DB blobs when absent)
    pub r2_account_id: Option<String>,
    pub r2_access_key_id: Option<String>,
    pub r2_secret_access_key: Option<String>,
    pub r2_bucket_name: Option<String>,
}

impl ServerConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let oauth = match (
            std::env::var("GITHUB_CLIENT_ID").ok().filter(|s| !s.is_empty()),
            std::env::var("GITHUB_CLIENT_SECRET").ok().filter(|s| !s.is_empty()),
        ) {
            (Some(client_id), Some(client_secret)) => Some(OAuthConfig {
                github_client_id: client_id,
                github_client_secret: client_secret,
                github_redirect_uri: env_or(
                    "GITHUB_REDIRECT_URI",
                    "http://localhost:5173/auth/github/callback",
                ),
            }),
            _ => None,
        };

        Self {
            host: env_or("GT_HOST", "0.0.0.0"),
            port: env_or("GT_PORT", "3001").parse().unwrap_or(3001),
            auth: AuthConfig {
                jwt_secret: env_or(
                    "GT_JWT_SECRET",
                    "globaltelco-dev-secret-change-in-production",
                ),
                access_token_expiry_secs: env_or("GT_ACCESS_TOKEN_EXPIRY", "3600")
                    .parse()
                    .unwrap_or(3600),
                refresh_token_expiry_secs: env_or("GT_REFRESH_TOKEN_EXPIRY", "2592000")
                    .parse()
                    .unwrap_or(2592000),
            },
            database_url: std::env::var("DATABASE_URL").ok(),
            default_world_name: env_or("GT_DEFAULT_WORLD", "Default World"),
            default_max_players: env_or("GT_MAX_PLAYERS", "8").parse().unwrap_or(8),
            tile_dir: std::env::var("TILE_DIR").ok().filter(|s| !s.is_empty()),
            oauth,
            cf_reset_worker_url: std::env::var("CF_RESET_WORKER_URL")
                .ok()
                .filter(|s| !s.is_empty()),
            r2_account_id: std::env::var("R2_ACCOUNT_ID").ok().filter(|s| !s.is_empty()),
            r2_access_key_id: std::env::var("R2_ACCESS_KEY_ID").ok().filter(|s| !s.is_empty()),
            r2_secret_access_key: std::env::var("R2_SECRET_ACCESS_KEY").ok().filter(|s| !s.is_empty()),
            r2_bucket_name: std::env::var("R2_BUCKET_NAME").ok().filter(|s| !s.is_empty()),
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
