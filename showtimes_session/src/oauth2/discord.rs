use std::sync::Arc;

use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://discord.com/api/v10";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub bot: Option<bool>,
    pub system: Option<bool>,
    pub mfa_enabled: Option<bool>,
    pub locale: Option<String>,
    pub verified: Option<bool>,
    pub email: Option<String>,
    pub flags: Option<u64>,
    pub premium_type: Option<u64>,
    pub public_flags: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordPartialGuild {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub owner: bool,
    pub permissions: String,
    pub features: Vec<String>,
}

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

#[derive(Debug)]
pub enum DiscordClientError {
    Reqwest(reqwest::Error),
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

    pub async fn get_user(&self, token: impl Into<String>) -> Result<DiscordUser, reqwest::Error> {
        let res = self
            .client
            .get(format!("{}/users/@me", BASE_URL))
            .header("Authorization", format!("Bearer {}", token.into()))
            .send()
            .await?;

        res.json().await
    }

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
