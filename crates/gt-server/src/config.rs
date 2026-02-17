use crate::auth::AuthConfig;

/// Server configuration, loaded from environment variables
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub auth: AuthConfig,
    pub database_url: Option<String>,
    pub default_world_name: String,
    pub default_max_players: u32,
}

impl ServerConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
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
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
