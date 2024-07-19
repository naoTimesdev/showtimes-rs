use bson::serde_helpers::chrono_datetime_as_bson_datetime;
use serde::{Deserialize, Serialize};
use showtimes_shared::{generate_uuid, ulid_serializer};

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
    #[serde(with = "ulid_serializer", default = "ulid_serializer::default")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<mongodb::bson::oid::ObjectId>,
    #[serde(
        with = "chrono_datetime_as_bson_datetime",
        default = "chrono::Utc::now"
    )]
    pub created: chrono::DateTime<chrono::Utc>,
    #[serde(
        with = "chrono_datetime_as_bson_datetime",
        default = "chrono::Utc::now"
    )]
    pub updated: chrono::DateTime<chrono::Utc>,
}

impl User {
    /// Create a new user
    pub fn new(username: String, discord_meta: DiscordUser) -> Self {
        let now = chrono::Utc::now();

        Self {
            id: ulid_serializer::default(),
            username,
            avatar: None,
            api_key: generate_uuid().to_string(),
            kind: UserKind::User,
            discord_meta,
            _id: None,
            created: now,
            updated: now,
        }
    }

    /// Create a new admin user
    pub fn new_admin(username: String, discord_meta: DiscordUser) -> Self {
        let now = chrono::Utc::now();

        Self {
            id: ulid_serializer::default(),
            username,
            avatar: None,
            api_key: generate_uuid().to_string(),
            kind: UserKind::Admin,
            discord_meta,
            _id: None,
            created: now,
            updated: now,
        }
    }
}
