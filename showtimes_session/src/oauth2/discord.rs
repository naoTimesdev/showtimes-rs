//! Discord OAuth2 support for Showtimes.
//!
//! This intergrates with Discord for authentication purposes
//! and used as the main login feature for Showtimes.
use std::sync::Arc;

use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://discord.com/api/v10";

/// The Discord token received when exchanging code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordToken {
    /// Access token
    pub access_token: String,
    /// Token type, usually bearer
    pub token_type: String,
    /// When the token expires, in seconds
    pub expires_in: u64,
    /// Refresh token, used to get a new access token
    pub refresh_token: Option<String>,
    /// The scope of the token
    pub scope: String,
}

/// A minimal representation of a Discord user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordUser {
    /// The discord ID
    pub id: String,
    /// Discord username
    pub username: String,
    /// Discord discriminator, this is unused
    pub discriminator: String,
    /// The avatar of the user
    pub avatar: Option<String>,
    /// Is the user a bot?
    pub bot: Option<bool>,
    /// Is the user a system?
    pub system: Option<bool>,
    /// Do user has MFA enabled?
    pub mfa_enabled: Option<bool>,
    /// The selected locale for the user
    pub locale: Option<String>,
    /// Is the user has verified account?
    pub verified: Option<bool>,
    /// The email associated, only available with proper scopes
    pub email: Option<String>,
    /// The flags of the user, bitflags
    pub flags: Option<u64>,
    /// Premium type of the user, bitflags
    pub premium_type: Option<u64>,
    /// The public flags of the user, bitflags
    pub public_flags: Option<u64>,
}

/// A minimal representation of a Discord guild
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordPartialGuild {
    /// The ID of the guild
    pub id: String,
    /// The name of the guild
    pub name: String,
    /// The icon of the guild
    pub icon: Option<String>,
    /// Is the current authenticated user the owner?
    pub owner: bool,
    /// Permissions available for the user
    pub permissions: String,
    /// Features enabled in the server
    pub features: Vec<String>,
}

/// The main Discord client for OAuth2 purpose
#[derive(Clone)]
pub struct DiscordClient {
    client_id: String,
    client_secret: String,
    client: Arc<reqwest::Client>,
}

impl std::fmt::Debug for DiscordClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscrodClient")
            .field("client_id", &self.client_id)
            .finish()
    }
}

/// A discord client Error
#[derive(Debug)]
pub enum DiscordClientError {
    /// An error occurred when requesting
    Reqwest(reqwest::Error),
    /// An error occurred when deserializing data
    Serde(serde_json::Error),
}

impl std::fmt::Display for DiscordClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscordClientError::Reqwest(e) => write!(f, "Reqwest error: {}", e),
            DiscordClientError::Serde(e) => write!(f, "Serde error: {}", e),
        }
    }
}

impl DiscordClient {
    /// Initiate a new client for OAuth2 via Discord
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        let client = reqwest::ClientBuilder::new()
            .user_agent(format!(
                "showtimes-rs-session/{} (+https://github.com/naoTimesdev/showtimes-rs)",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .expect("Failed to build reqwest client for Discord OAuth2");

        Self {
            client: Arc::new(client),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
        }
    }

    /// Exchange code received from callback with proper OAuth2 token
    pub async fn exchange_code(
        &self,
        code: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Result<DiscordToken, DiscordClientError> {
        let res = self
            .client
            .post(format!("{}/oauth2/token", BASE_URL))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
                ("grant_type", &"authorization_code".to_string()),
                ("code", &code.into()),
                ("redirect_uri", &redirect_uri.into()),
            ])
            .send()
            .await
            .map_err(DiscordClientError::Reqwest)?;

        let raw_resp = res.text().await.map_err(DiscordClientError::Reqwest)?;

        serde_json::from_str::<DiscordToken>(&raw_resp).map_err(|e| {
            println!("Error: {:?}", e);
            println!("Body: {:?}", raw_resp);

            DiscordClientError::Serde(e)
        })
    }

    /// Refresh the access token with a new one via refresh token grant type
    pub async fn refresh_token(
        &self,
        refresh_token: impl Into<String>,
    ) -> Result<DiscordToken, reqwest::Error> {
        let res = self
            .client
            .post(format!("{}/oauth2/token", BASE_URL))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
                ("grant_type", &"refresh_token".to_string()),
                ("refresh_token", &refresh_token.into()),
            ])
            .send()
            .await?;

        res.json().await
    }

    /// Get user information of the current user.
    pub async fn get_user(&self, token: impl Into<String>) -> Result<DiscordUser, reqwest::Error> {
        let res = self
            .client
            .get(format!("{}/users/@me", BASE_URL))
            .header("Authorization", format!("Bearer {}", token.into()))
            .send()
            .await?;

        res.json().await
    }

    /// Get guilds list of the current user.
    pub async fn get_guilds(
        &self,
        token: impl Into<String>,
    ) -> Result<Vec<DiscordPartialGuild>, reqwest::Error> {
        let res = self
            .client
            .get(format!("{}/users/@me/guilds", BASE_URL))
            .header("Authorization", format!("Bearer {}", token.into()))
            .send()
            .await?;

        res.json().await
    }
}
