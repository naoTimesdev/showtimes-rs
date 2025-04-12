use serde::{Deserialize, Serialize};
use showtimes_derive::EnumName;
use showtimes_shared::{ulid_opt_serializer, ulid_serializer};

use super::{ImageMetadata, IntegrationId, ShowModelHandler};

/// Enum to hold user privileges on a server.
///
/// There is no "normal" user, as all users are considered normal.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, EnumName)]
#[enum_name(rename_all = "kebab-case")]
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
    /// - Manage the collaboration
    Manager,
    /// A user with all the special privileges
    ///
    /// In addition to manager, this user can:
    /// - Add and remove users
    /// - Manage the server settings
    /// - Manage RSS feeds
    Admin,
    /// A user with complete control over the server
    ///
    /// In addition to admin, this user can:
    /// - Delete the server
    /// - Add or remove admins
    ///
    /// Only one user can have this privilege
    Owner,
}

impl std::fmt::Display for UserPrivilege {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserPrivilege::ProjectManager => write!(f, "Project Manager"),
            UserPrivilege::Manager => write!(f, "Manager"),
            UserPrivilege::Admin => write!(f, "Admin"),
            UserPrivilege::Owner => write!(f, "Owner"),
        }
    }
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

    pub fn has_id(&self, id: impl Into<String>) -> bool {
        let into_id = id.into();
        self.extras.contains(&into_id)
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

impl Server {
    pub fn new(name: impl Into<String>, owners: Vec<ServerUser>) -> Self {
        let cur_time = jiff::Timestamp::now();
        Server {
            id: ulid_serializer::default(),
            name: name.into(),
            integrations: Vec::new(),
            owners,
            avatar: None,
            _id: None,
            created: cur_time,
            updated: cur_time,
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

    pub fn add_integration(&mut self, integration: IntegrationId) {
        self.integrations.push(integration);
    }

    pub fn remove_integration(&mut self, integration: &IntegrationId) {
        self.integrations.retain(|i| i != integration);
    }

    pub fn add_owner(&mut self, owner: ServerUser) {
        self.owners.push(owner);
    }

    /// Remove an owner from the server
    ///
    /// This will silently fails if the owner is not found
    /// or you're trying to remove the owner
    pub fn remove_owner(&mut self, integration: &showtimes_shared::ulid::Ulid) {
        self.owners
            .retain(|i| i.id != *integration && i.privilege != UserPrivilege::Owner);
    }
}

/// A model to hold each server that is synchronized
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ServerCollaborationSyncTarget {
    /// The server ID
    #[serde(with = "ulid_serializer")]
    pub server: showtimes_shared::ulid::Ulid,
    /// The project ID
    #[serde(with = "ulid_serializer")]
    pub project: showtimes_shared::ulid::Ulid,
}

impl ServerCollaborationSyncTarget {
    pub fn new(
        server: showtimes_shared::ulid::Ulid,
        project: showtimes_shared::ulid::Ulid,
    ) -> Self {
        ServerCollaborationSyncTarget { server, project }
    }
}

impl From<super::Project> for ServerCollaborationSyncTarget {
    fn from(value: super::Project) -> Self {
        ServerCollaborationSyncTarget::new(value.creator, value.id)
    }
}

impl From<&super::Project> for ServerCollaborationSyncTarget {
    fn from(value: &super::Project) -> Self {
        ServerCollaborationSyncTarget::new(value.creator, value.id)
    }
}

/// A model to hold server synchronization information on a project
#[derive(Debug, Clone, Serialize, Deserialize, showtimes_derive::ShowModelHandler)]
#[col_name("ShowtimesCollaborationSync")]
pub struct ServerCollaborationSync {
    /// The collaboration ID
    #[serde(with = "ulid_serializer")]
    pub id: showtimes_shared::ulid::Ulid,
    /// The list of projects target
    pub projects: Vec<ServerCollaborationSyncTarget>,
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

impl ServerCollaborationSync {
    pub fn new(projects: Vec<ServerCollaborationSyncTarget>) -> Self {
        let cur_time = jiff::Timestamp::now();
        ServerCollaborationSync {
            id: ulid_serializer::default(),
            projects,
            _id: None,
            created: cur_time,
            updated: cur_time,
        }
    }

    /// Get a specific project and remove it from the list
    pub fn get_and_remove(
        &mut self,
        project: showtimes_shared::ulid::Ulid,
    ) -> Option<ServerCollaborationSyncTarget> {
        let index = self.projects.iter().position(|p| p.project == project);
        if let Some(index) = index {
            Some(self.projects.remove(index))
        } else {
            None
        }
    }

    /// Get a specific server and remove it from the list
    pub fn get_and_remove_server(
        &mut self,
        server: showtimes_shared::ulid::Ulid,
    ) -> Option<ServerCollaborationSyncTarget> {
        let index = self.projects.iter().position(|p| p.server == server);
        if let Some(index) = index {
            Some(self.projects.remove(index))
        } else {
            None
        }
    }

    /// Get the length of the projects or synchronization list
    pub fn length(&self) -> usize {
        self.projects.len()
    }
}

/// An information for a collaboration invite (for source)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ServerCollaborationInviteSource {
    /// The server ID
    #[serde(with = "ulid_serializer")]
    pub server: showtimes_shared::ulid::Ulid,
    /// The project ID
    #[serde(with = "ulid_serializer")]
    pub project: showtimes_shared::ulid::Ulid,
}

impl ServerCollaborationInviteSource {
    pub fn new(
        server: showtimes_shared::ulid::Ulid,
        project: showtimes_shared::ulid::Ulid,
    ) -> Self {
        ServerCollaborationInviteSource { server, project }
    }
}

/// An information for a collaboration invite (for target)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ServerCollaborationInviteTarget {
    /// The server ID
    #[serde(with = "ulid_serializer")]
    pub server: showtimes_shared::ulid::Ulid,
    /// The project ID (can be `None`)
    ///
    /// If `None` then the source server data
    /// will be used as the project data
    #[serde(with = "ulid_opt_serializer")]
    pub project: Option<showtimes_shared::ulid::Ulid>,
}

impl ServerCollaborationInviteTarget {
    pub fn new(server: showtimes_shared::ulid::Ulid) -> Self {
        ServerCollaborationInviteTarget {
            server,
            project: None,
        }
    }

    pub fn new_with_project(
        server: showtimes_shared::ulid::Ulid,
        project: showtimes_shared::ulid::Ulid,
    ) -> Self {
        ServerCollaborationInviteTarget {
            server,
            project: Some(project),
        }
    }
}

/// A model to hold server collaboration invite on a project
#[derive(Debug, Clone, Serialize, Deserialize, showtimes_derive::ShowModelHandler)]
#[col_name("ShowtimesCollaborationInvite")]
pub struct ServerCollaborationInvite {
    /// The collab invite ID (unique, and used as invite code too)
    #[serde(with = "ulid_serializer")]
    pub id: showtimes_shared::ulid::Ulid,
    /// The source server
    pub source: ServerCollaborationInviteSource,
    /// The target server
    pub target: ServerCollaborationInviteTarget,
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

impl ServerCollaborationInvite {
    pub fn new(
        source: ServerCollaborationInviteSource,
        target: ServerCollaborationInviteTarget,
    ) -> Self {
        let cur_time = jiff::Timestamp::now();
        ServerCollaborationInvite {
            id: ulid_serializer::default(),
            source,
            target,
            _id: None,
            created: cur_time,
            updated: cur_time,
        }
    }
}

/// A model to hold premium usages informaiton
#[derive(Debug, Clone, Serialize, Deserialize, showtimes_derive::ShowModelHandler)]
#[col_name("ShowtimesServerPremium")]
pub struct ServerPremium {
    /// The premium ID
    #[serde(with = "ulid_serializer")]
    pub id: showtimes_shared::ulid::Ulid,
    /// The target server
    #[serde(with = "ulid_serializer")]
    pub target: showtimes_shared::ulid::Ulid,
    /// The premium end date
    #[serde(with = "showtimes_shared::bson_datetime_jiff_timestamp")]
    pub ends_at: jiff::Timestamp,
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

impl ServerPremium {
    /// Create a new instance of [`ServerPremium`].
    pub fn new(target: showtimes_shared::ulid::Ulid, ends_at: jiff::Timestamp) -> Self {
        let cur_time = jiff::Timestamp::now();
        ServerPremium {
            id: ulid_serializer::default(),
            target,
            ends_at,
            _id: None,
            created: cur_time,
            updated: cur_time,
        }
    }

    /// Extend the end date to the given date.
    pub fn extend_at(mut self, ends_at: jiff::Timestamp) -> Self {
        self.ends_at = ends_at;
        self
    }

    /// Extend the end date by the given duration.
    ///
    /// Returns `self` for chainability.
    pub fn extend_by(mut self, duration: jiff::Span) -> Self {
        self.ends_at += duration;
        self
    }
}
