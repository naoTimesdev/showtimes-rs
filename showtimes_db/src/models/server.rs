use bson::serde_helpers::chrono_datetime_as_bson_datetime;
use serde::{Deserialize, Serialize};
use showtimes_shared::{ulid_list_serializer, ulid_opt_serializer, ulid_serializer};

use super::{ImageMetadata, IntegrationId, ShowModelHandler};

/// Enum to hold user privileges on a server.
///
/// There is no "normal" user, as all users are considered normal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserPrivilege {
    /// A project manager on a server
    ///
    /// This user can:
    /// - Manage single or multiple projects
    ProjectManager,
    /// A manager of the server
    ///
    /// In addition to project manager, this user can:
    /// - Add and remove project
    /// - Manage all project
    Manager,
    /// A user with all the special privileges
    ///
    /// In addition to manager, this user can:
    /// - Add and remove users
    /// - Manage the server settings
    Admin,
}

/// A model to hold user information on a server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerUser {
    /// The associated user ID
    #[serde(with = "ulid_serializer")]
    pub id: showtimes_shared::ulid::Ulid,
    /// The user's privilege
    pub privilege: UserPrivilege,
    /// The extra associated data with the user
    ///
    /// Used to store extra data like what projects the user is managing
    pub extras: Vec<String>,
}

impl ServerUser {
    /// Create a new server user
    pub fn new(id: showtimes_shared::ulid::Ulid, privilege: UserPrivilege) -> Self {
        ServerUser {
            id,
            privilege,
            extras: Vec::new(),
        }
    }
}

/// A model to hold server information
///
/// The original account is called "server" as a caddy over from the original
/// project. This is a server in the sense of a project server, not a physical
#[derive(Debug, Clone, Serialize, Deserialize, showtimes_derive::ShowModelHandler)]
#[col_name("ShowtimesServers")]
pub struct Server {
    /// The server's ID
    #[serde(with = "ulid_serializer")]
    pub id: showtimes_shared::ulid::Ulid,
    /// The server's name
    pub name: String,
    /// The server's integrations
    pub integrations: Vec<IntegrationId>,
    /// The server's owners
    pub owners: Vec<ServerUser>,
    /// The server's avatar/icon
    pub avatar: Option<ImageMetadata>,
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

impl Server {
    pub fn new(name: impl Into<String>, owners: Vec<ServerUser>) -> Self {
        Server {
            id: showtimes_shared::ulid::Ulid::new(),
            name: name.into(),
            integrations: Vec::new(),
            owners,
            avatar: None,
            _id: None,
            created: chrono::Utc::now(),
            updated: chrono::Utc::now(),
        }
    }

    pub fn with_avatar(mut self, avatar: ImageMetadata) -> Self {
        self.avatar = Some(avatar);
        self
    }

    pub fn with_integration(mut self, integration: IntegrationId) -> Self {
        self.integrations.push(integration);
        self
    }

    pub fn with_integrations(mut self, integrations: Vec<IntegrationId>) -> Self {
        self.integrations = integrations;
        self
    }
}

/// A model to hold server synchronization information on a project
#[derive(Debug, Clone, Serialize, Deserialize, showtimes_derive::ShowModelHandler)]
#[col_name("ShowtimesCollaborationSync")]
pub struct ServerCollaborationSync {
    /// The collaboration ID
    #[serde(with = "ulid_serializer")]
    pub id: showtimes_shared::ulid::Ulid,
    /// The list of projects
    #[serde(with = "ulid_list_serializer")]
    pub projects: Vec<showtimes_shared::ulid::Ulid>,
    /// The list of servers
    #[serde(with = "ulid_list_serializer")]
    pub servers: Vec<showtimes_shared::ulid::Ulid>,
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

/// An information for a collaboration invite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCollaborationInviteInfo {
    /// The server ID
    #[serde(with = "ulid_serializer")]
    pub server: showtimes_shared::ulid::Ulid,
    /// The project ID (can be null)
    #[serde(with = "ulid_opt_serializer")]
    pub project: Option<showtimes_shared::ulid::Ulid>,
}

/// A model to hold server collaboration invite on a project
#[derive(Debug, Clone, Serialize, Deserialize, showtimes_derive::ShowModelHandler)]
#[col_name("ShowtimesCollaborationInvite")]
pub struct ServerCollaborationInvite {
    /// The collab invite ID (unique, and used as invite code too)
    #[serde(with = "ulid_serializer")]
    pub id: showtimes_shared::ulid::Ulid,
    /// The source server
    pub source: ServerCollaborationInviteInfo,
    /// The target server
    pub target: ServerCollaborationInviteInfo,
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
