use serde::{Deserialize, Serialize};
use showtimes_shared::{de_opt_ulid, de_ulid, de_ulid_list, ser_opt_ulid, ser_ulid, ser_ulid_list};

use super::{ImageMetadata, IntegrationId};

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
    #[serde(serialize_with = "ser_ulid", deserialize_with = "de_ulid")]
    pub id: showtimes_shared::ulid::Ulid,
    /// The user's privilege
    pub privilege: UserPrivilege,
    /// The extra associated data with the user
    ///
    /// Used to store extra data like what projects the user is managing
    pub extras: Vec<String>,
}

/// A model to hold server information
///
/// The original account is called "server" as a caddy over from the original
/// project. This is a server in the sense of a project server, not a physical
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    /// The server's ID
    #[serde(serialize_with = "ser_ulid", deserialize_with = "de_ulid")]
    pub id: showtimes_shared::ulid::Ulid,
    /// The server's name
    pub name: String,
    /// The server's integrations
    pub integrations: Vec<IntegrationId>,
    /// The server's owners
    pub owners: Vec<ServerUser>,
    /// The server's avatar/icon
    pub avatar: Option<ImageMetadata>,
}

/// A model to hold server synchronization information on a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCollaborationSync {
    /// The collaboration ID
    #[serde(serialize_with = "ser_ulid", deserialize_with = "de_ulid")]
    pub id: showtimes_shared::ulid::Ulid,
    /// The list of projects
    #[serde(serialize_with = "ser_ulid_list", deserialize_with = "de_ulid_list")]
    pub projects: Vec<showtimes_shared::ulid::Ulid>,
    /// The list of servers
    #[serde(serialize_with = "ser_ulid_list", deserialize_with = "de_ulid_list")]
    pub servers: Vec<showtimes_shared::ulid::Ulid>,
}

/// An information for a collaboration invite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCollaborationInviteInfo {
    /// The server ID
    #[serde(serialize_with = "ser_ulid", deserialize_with = "de_ulid")]
    pub server: showtimes_shared::ulid::Ulid,
    /// The project ID (can be null)
    #[serde(serialize_with = "ser_opt_ulid", deserialize_with = "de_opt_ulid")]
    pub project: Option<showtimes_shared::ulid::Ulid>,
}

/// A model to hold server collaboration invite on a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCollaborationInvite {
    /// The collab invite ID (unique, and used as invite code too)
    #[serde(serialize_with = "ser_ulid", deserialize_with = "de_ulid")]
    pub id: showtimes_shared::ulid::Ulid,
    /// The source server
    pub source: ServerCollaborationInviteInfo,
    /// The target server
    pub target: ServerCollaborationInviteInfo,
}
