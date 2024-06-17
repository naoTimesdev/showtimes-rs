use serde::{Deserialize, Serialize};
use showtimes_shared::{de_ulid, def_ulid, generate_uuid, ser_ulid};

use super::ImageMetadata;

/// Enum to hold user kinds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserKind {
    /// A normal user
    User,
    /// An admin user
    Admin,
}

/// A model to hold discord user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordUser {
    /// The user's ID
    pub id: String,
    /// The user's username
    pub username: String,
    /// The user's avatar
    pub avatar: Option<String>,
    /// The user access token
    pub access_token: String,
    /// The user refresh token
    pub refresh_token: String,
    /// The user expires at
    pub expires_at: i64,
}

/// A model to hold user authentication information
///
/// User is logged in via Discord OAuth2
#[derive(Debug, Clone, Serialize, Deserialize, showtimes_derive::ShowModelHandler)]
#[col_name("ShowtimesUsers")]
pub struct User {
    /// The user's ID
    #[serde(
        serialize_with = "ser_ulid",
        deserialize_with = "de_ulid",
        default = "def_ulid"
    )]
    pub id: showtimes_shared::ulid::Ulid,
    /// The user's username
    ///
    /// This can be changed by the user.
    pub username: String,
    /// The user's avatar
    ///
    /// This can be changed by the user.
    pub avatar: Option<ImageMetadata>,
    /// The user API key
    pub api_key: String,
    /// The user kind
    pub kind: UserKind,
    /// The user discord information
    pub discord_meta: DiscordUser,
    #[serde(skip_serializing)]
    _id: Option<mongodb::bson::oid::ObjectId>,
}

impl User {
    /// Create a new user
    pub fn new(username: String, discord_meta: DiscordUser) -> Self {
        Self {
            id: def_ulid(),
            username,
            avatar: None,
            api_key: generate_uuid().to_string(),
            kind: UserKind::User,
            discord_meta,
            _id: None,
        }
    }

    /// Create a new admin user
    pub fn new_admin(username: String, discord_meta: DiscordUser) -> Self {
        Self {
            id: def_ulid(),
            username,
            avatar: None,
            api_key: generate_uuid().to_string(),
            kind: UserKind::Admin,
            discord_meta,
            _id: None,
        }
    }
}
