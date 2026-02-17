use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{self, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // player UUID
    pub username: String,
    pub exp: usize, // expiry (unix timestamp)
    pub iat: usize, // issued at
    pub is_guest: bool,
}

/// Auth configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_expiry_secs: u64,
    pub refresh_token_expiry_secs: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "globaltelco-dev-secret-change-in-production".to_string(),
            access_token_expiry_secs: 3600,        // 1 hour
            refresh_token_expiry_secs: 86400 * 30, // 30 days
        }
    }
}

/// Hash a password using Argon2
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Generate a JWT access token
pub fn generate_access_token(
    config: &AuthConfig,
    player_id: Uuid,
    username: &str,
    is_guest: bool,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: player_id.to_string(),
        username: username.to_string(),
        exp: now + config.access_token_expiry_secs as usize,
        iat: now,
        is_guest,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
}

/// Generate a refresh token (longer-lived JWT)
pub fn generate_refresh_token(
    config: &AuthConfig,
    player_id: Uuid,
    username: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: player_id.to_string(),
        username: username.to_string(),
        exp: now + config.refresh_token_expiry_secs as usize,
        iat: now,
        is_guest: false,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
}

/// Validate a JWT token and return claims
pub fn validate_token(
    config: &AuthConfig,
    token: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}
