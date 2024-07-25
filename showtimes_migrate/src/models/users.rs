use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordMeta {
    pub id: String,
    pub name: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

/// The user model, mapped into `showtimesuilogin`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    // This is password basically
    pub secret: String,
    pub name: Option<String>,
    // owner, server (standard server user)
    pub privilege: String,
    #[serde(default)]
    pub discord_meta: Option<DiscordMeta>,
    // DISCORD or PASSWORD
    #[serde(default)]
    pub user_type: Option<String>,
}
