//! GitHub OAuth integration.
//!
//! Exchange authorization codes for access tokens, then fetch user profiles.
//! Only compiled when the `oauth` feature is enabled.

#[cfg(feature = "oauth")]
use serde::Deserialize;

/// GitHub OAuth configuration
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub github_client_id: String,
    pub github_client_secret: String,
    pub github_redirect_uri: String,
}

/// GitHub user profile returned from the API
#[cfg(feature = "oauth")]
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

/// Exchange a GitHub authorization code for an access token, then fetch user profile.
#[cfg(feature = "oauth")]
pub async fn github_exchange(config: &OAuthConfig, code: &str) -> Result<GitHubUser, String> {
    let client = reqwest::Client::new();

    // Step 1: Exchange code for access token
    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: String,
    }

    let token_resp = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "client_id": config.github_client_id,
            "client_secret": config.github_client_secret,
            "code": code,
            "redirect_uri": config.github_redirect_uri,
        }))
        .send()
        .await
        .map_err(|e| format!("GitHub token exchange failed: {e}"))?;

    if !token_resp.status().is_success() {
        let status = token_resp.status();
        let body = token_resp.text().await.unwrap_or_default();
        return Err(format!("GitHub token exchange returned {status}: {body}"));
    }

    let token: TokenResponse = token_resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse GitHub token response: {e}"))?;

    // Step 2: Fetch user profile
    let user_resp = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", token.access_token))
        .header("User-Agent", "GlobalTelco-Server")
        .send()
        .await
        .map_err(|e| format!("GitHub user fetch failed: {e}"))?;

    if !user_resp.status().is_success() {
        let status = user_resp.status();
        let body = user_resp.text().await.unwrap_or_default();
        return Err(format!("GitHub user API returned {status}: {body}"));
    }

    let user: GitHubUser = user_resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse GitHub user: {e}"))?;

    Ok(user)
}
