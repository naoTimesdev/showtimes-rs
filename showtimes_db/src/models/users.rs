use jwt_lc_rs::SigningAlgorithm;
use serde::{Deserialize, Serialize};
use showtimes_derive::EnumName;
use showtimes_shared::ulid_serializer;

use crate::impl_trait_model;

use super::{ImageMetadata, ShowModelHandler};

/// Enum to hold user kinds
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, PartialOrd, Eq, Ord, EnumName,
)]
pub enum UserKind {
    /// A normal user
    #[default]
    User,
    /// An admin user, can see all users and manage all servers
    Admin,
    /// "Owner" user, basically can do anything
    ///
    /// This is a non-existent role but used internally
    /// to mark that this is request made by master key
    /// which can do anything.
    Owner,
}

/// Enum that say how the API key should be used (or the limitation)
///
/// This API key is still locked to the user kind and the user itself
/// so if you create API key with specific or greater user case, it will
/// not be able to do anything.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord, Hash, EnumName,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[enum_name(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum APIKeyCapability {
    /// A combination of create and update for servers
    ManageServers,
    /// A combination of create and update for projects
    ManageProjects,
    /// A combination of create, update, and delete for RSS feeds
    #[serde(rename = "MANAGE_RSS")]
    #[enum_name(rename = "MANAGE_RSS")]
    ManageRSS,
    /// A combination of update for users
    ManageUsers,
    /// A delete capability for servers
    DeleteServers,
    /// A delete capability for projects
    DeleteProjects,
    /// Manage collaboration of a project
    ///
    /// This can do everything of collaboration thing does
    ManageCollaboration,
    /// Query for servers
    QueryServers,
    /// Query for projects
    QueryProjects,
    /// Query for stats
    QueryStats,
    /// Query for search data
    QuerySearch,
}

impl APIKeyCapability {
    /// Get all capabilities
    pub fn all() -> &'static [APIKeyCapability] {
        &[
            APIKeyCapability::ManageServers,
            APIKeyCapability::ManageProjects,
            APIKeyCapability::ManageRSS,
            APIKeyCapability::ManageUsers,
            APIKeyCapability::DeleteServers,
            APIKeyCapability::DeleteProjects,
            APIKeyCapability::ManageCollaboration,
            APIKeyCapability::QueryServers,
            APIKeyCapability::QueryProjects,
            APIKeyCapability::QueryStats,
            APIKeyCapability::QuerySearch,
        ]
    }

    /// Get all capabilities for query only operation
    pub fn queries() -> &'static [APIKeyCapability] {
        &[
            APIKeyCapability::QueryServers,
            APIKeyCapability::QueryProjects,
            APIKeyCapability::QueryStats,
            APIKeyCapability::QuerySearch,
        ]
    }

    /// Get all capabilities for management operation
    pub fn manages() -> &'static [APIKeyCapability] {
        &[
            APIKeyCapability::ManageServers,
            APIKeyCapability::ManageProjects,
            APIKeyCapability::ManageRSS,
            APIKeyCapability::ManageUsers,
        ]
    }

    /// Get all capabilities for deletion operation
    pub fn deletes() -> &'static [APIKeyCapability] {
        &[
            APIKeyCapability::DeleteServers,
            APIKeyCapability::DeleteProjects,
        ]
    }

    /// Get all operation related to projects
    pub fn projects() -> &'static [APIKeyCapability] {
        &[
            APIKeyCapability::ManageProjects,
            APIKeyCapability::DeleteProjects,
            APIKeyCapability::QueryProjects,
        ]
    }

    /// Get all operation related to servers
    pub fn servers() -> &'static [APIKeyCapability] {
        &[
            APIKeyCapability::ManageServers,
            APIKeyCapability::DeleteServers,
            APIKeyCapability::QueryServers,
        ]
    }

    /// Get all operation related to RSS feeds
    pub fn rss() -> &'static [APIKeyCapability] {
        &[APIKeyCapability::ManageRSS]
    }

    /// Get all operation related to users
    pub fn users() -> &'static [APIKeyCapability] {
        &[APIKeyCapability::ManageUsers]
    }
}

/// A model to hold API key information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIKey {
    /// The API key itself
    pub key: APIKeyHashed,
    /// The API key capabilities
    pub capabilities: Vec<APIKeyCapability>,
}

impl APIKey {
    /// Create a new API key with the given key and capabilities
    ///
    /// The provided capabilities should be a subset of the capabilities
    /// returned by `APIKeyCapability::all()`.
    pub fn new(key: showtimes_shared::APIKey, capabilities: Vec<APIKeyCapability>) -> Self {
        APIKey {
            key: key.into(),
            capabilities,
        }
    }

    /// Check if API key has specific capability
    pub fn can(&self, capability: APIKeyCapability) -> bool {
        self.capabilities.contains(&capability)
    }

    /// Check if API key has the following capabilities
    pub fn can_all(&self, capabilities: &[APIKeyCapability]) -> bool {
        capabilities.iter().all(|c| self.can(*c))
    }

    /// Check if API key has any of the following capabilities
    pub fn can_any(&self, capabilities: &[APIKeyCapability]) -> bool {
        capabilities.iter().any(|c| self.can(*c))
    }

    /// Stub an API key
    pub fn stub() -> Self {
        APIKey {
            key: showtimes_shared::APIKey::new().into(),
            capabilities: Vec::new(),
        }
    }

    /// Update API key
    pub fn update_key<T: Into<APIKeyHashed>>(&mut self, key: T) {
        self.key = key.into();
    }
}

impl Default for APIKey {
    fn default() -> Self {
        APIKey {
            key: showtimes_shared::APIKey::new().into(),
            capabilities: APIKeyCapability::all().to_vec(),
        }
    }
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

    /// Stub a discord user with specific ID
    ///
    /// # Note
    /// This only be used when orchestrating as a specific user and it doens't registered
    pub fn stub_with_id(id: impl Into<String>) -> Self {
        DiscordUser {
            id: id.into(),
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub api_key: Vec<APIKey>,
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
        with = "showtimes_shared::bson_datetime_jiff_timestamp",
        default = "jiff::Timestamp::now"
    )]
    pub created: jiff::Timestamp,
    #[serde(
        with = "showtimes_shared::bson_datetime_jiff_timestamp",
        default = "jiff::Timestamp::now"
    )]
    pub updated: jiff::Timestamp,
}

impl_trait_model!(User, "ShowtimesUsers", _id, updated);

impl User {
    /// Create a new user
    pub fn new(username: String, discord_meta: DiscordUser) -> Self {
        let now = jiff::Timestamp::now();

        Self {
            id: ulid_serializer::default(),
            username,
            avatar: None,
            api_key: vec![APIKey::default()],
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
        let now = jiff::Timestamp::now();

        Self {
            id: ulid_serializer::default(),
            username,
            avatar: None,
            api_key: vec![APIKey::default()],
            kind: UserKind::Admin,
            registered: true,
            discord_meta,
            _id: None,
            created: now,
            updated: now,
        }
    }

    /// Stub a owner user
    pub fn stub_owner(master_key: impl Into<String>) -> Self {
        let now = jiff::Timestamp::now();

        let mut discord = DiscordUser::stub();
        discord.id = master_key.into();
        discord.username = "Showtimes Administrator".to_string();
        discord.access_token = "stub".to_string();
        discord.refresh_token = "stub".to_string();

        Self {
            id: ulid_serializer::default(),
            username: "Showtimes Administrator".to_string(),
            avatar: None,
            api_key: vec![APIKey::default()],
            kind: UserKind::Owner,
            registered: true,
            // Stub discord user since this is a master key
            discord_meta: discord,
            _id: None,
            created: now,
            updated: now,
        }
    }

    /// Stub a user
    pub fn stub() -> Self {
        let now = jiff::Timestamp::now();

        Self {
            id: ulid_serializer::default(),
            username: "Showtimes User".to_string(),
            avatar: None,
            api_key: vec![APIKey::stub()],
            kind: UserKind::User,
            registered: true,
            discord_meta: DiscordUser::stub(),
            _id: None,
            created: now,
            updated: now,
        }
    }

    /// Stub a user with specific ID
    pub fn stub_with_id(id: showtimes_shared::ulid::Ulid) -> Self {
        Self { id, ..Self::stub() }
    }

    /// Stub a user with specific Discord ID
    pub fn stub_with_discord_id(id: impl Into<String>) -> Self {
        Self {
            discord_meta: DiscordUser::stub_with_id(id),
            ..Self::stub()
        }
    }

    /// Create with unregistered status
    pub fn with_unregistered(&self) -> Self {
        Self {
            registered: false,
            ..self.clone()
        }
    }
}

#[derive(Debug, Clone)]
pub enum APIKeyHashed {
    Unhashed(showtimes_shared::APIKey),
    MaybeHashed(String),
}

impl Serialize for APIKeyHashed {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            APIKeyHashed::Unhashed(key) => {
                let hashed_key = hash_api_key(key).map_err(serde::ser::Error::custom)?;
                serializer.serialize_str(&hashed_key)
            }
            APIKeyHashed::MaybeHashed(key) => serializer.serialize_str(key),
        }
    }
}

impl From<showtimes_shared::APIKey> for APIKeyHashed {
    fn from(key: showtimes_shared::APIKey) -> Self {
        APIKeyHashed::Unhashed(key)
    }
}

impl<'de> Deserialize<'de> for APIKeyHashed {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let key = String::deserialize(deserializer)?;
        // check if startswith nsh_
        if key.starts_with("nsh_") {
            // try parsing as UUID
            let api_key =
                showtimes_shared::APIKey::from_string(key).map_err(serde::de::Error::custom)?;
            Ok(APIKeyHashed::Unhashed(api_key))
        } else {
            // if not, just return the string as is
            Ok(APIKeyHashed::MaybeHashed(key))
        }
    }
}

impl PartialEq<showtimes_shared::APIKey> for APIKeyHashed {
    fn eq(&self, other: &showtimes_shared::APIKey) -> bool {
        match self {
            APIKeyHashed::Unhashed(key) => key == other,
            APIKeyHashed::MaybeHashed(key) => {
                let hashed_key = hash_api_key(other).unwrap_or_default();
                hashed_key == *key
            }
        }
    }
}

/// Hash the API key using SHA-384 with HMAC
///
/// The HMAC key is derived from the API key (a.k.a the UUID)
/// then both the `nsh_[uuid]` will get hashed with SHA-384
fn hash_api_key(api_key: &showtimes_shared::APIKey) -> Result<String, jwt_lc_rs::errors::Error> {
    let key = api_key.as_uuid().to_bytes_le();

    let hmac = jwt_lc_rs::HmacAlgorithm::new(jwt_lc_rs::SHALevel::SHA384, key);

    hmac.sign(api_key.as_api_key().as_bytes())
}
