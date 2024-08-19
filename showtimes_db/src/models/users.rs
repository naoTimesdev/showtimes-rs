use bson::serde_helpers::chrono_datetime_as_bson_datetime;
use serde::{Deserialize, Serialize};
use showtimes_shared::{generate_uuid, ulid_serializer};

use super::{ImageMetadata, ShowModelHandler};

/// Enum to hold user kinds
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, PartialOrd, Eq, Ord)]
pub enum UserKind {
    /// A normal user
    #[default]
    User,
    /// An admin user, can see all users and manage all servers
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

impl DiscordUser {
    /// Stub a discord user
    ///
    /// # Note
    /// This is used only for migrations.
    pub fn stub() -> Self {
        DiscordUser {
            id: String::new(),
            username: String::new(),
            avatar: None,
            access_token: String::new(),
            refresh_token: String::new(),
            expires_at: -1,
        }
    }
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
    /// Check if the user registered, this is used to verify
    /// data from old migrations
    pub registered: bool,
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
            registered: true,
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
            registered: true,
            discord_meta,
            _id: None,
            created: now,
            updated: now,
        }
    }

    pub fn with_unregistered(&self) -> Self {
        Self {
            registered: false,
            ..self.clone()
        }
    }
}
