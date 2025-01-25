//! The models collection for Showtimes Search data.
//!
//! Powered by Meilisearch

use serde::{Deserialize, Serialize};
use showtimes_derive::SearchModel;
use showtimes_shared::{ulid_list_serializer, ulid_opt_serializer, ulid_serializer};
use std::ops::Deref;

/// The project model for Showtimes Search data.
///
/// This is mapped from [`showtimes_db::m::Project`] and use `nt-projects` as the index name.
#[derive(Debug, Clone, Serialize, Deserialize, Default, SearchModel)]
#[search(
    name = "nt-projects",
    filterable = ["id", "parent", "created", "title", "aliases", "kind", "status", "integrations.id", "integrations.kind"],
    searchable = ["id", "title", "aliases", "parent", "integrations.id"],
    sortable = ["id", "created", "updated"],
    distinct = "id",
)]
pub struct Project {
    /// The unique identifier of the project
    #[serde(with = "ulid_serializer", default = "ulid_serializer::default")]
    #[primary_key]
    pub id: showtimes_shared::ulid::Ulid,
    /// The title of the project
    pub title: String,
    /// The poster URL of the project
    pub poster_url: Option<String>,
    /// The integrations of the project
    pub integrations: Vec<showtimes_db::models::IntegrationId>,
    /// The aliases of the project
    pub aliases: Vec<String>,
    /// The parent server or creator
    #[serde(with = "ulid_serializer", default = "ulid_serializer::default")]
    pub parent: showtimes_shared::ulid::Ulid,
    /// The status of the project
    pub status: showtimes_db::m::ProjectStatus,
    /// The type of the project
    pub kind: showtimes_db::m::ProjectType,
    /// The date and time the project was created
    #[serde(
        with = "showtimes_shared::unix_timestamp_serializer",
        default = "chrono::Utc::now"
    )]
    pub created: chrono::DateTime<chrono::Utc>,
    /// The date and time the project was last updated
    #[serde(
        with = "showtimes_shared::unix_timestamp_serializer",
        default = "chrono::Utc::now"
    )]
    pub updated: chrono::DateTime<chrono::Utc>,
}

impl From<showtimes_db::m::Project> for Project {
    fn from(value: showtimes_db::m::Project) -> Self {
        let poster_url = value.poster.image.as_url();
        Self {
            id: value.id,
            title: value.title,
            poster_url: Some(poster_url),
            integrations: value.integrations,
            aliases: value.aliases,
            parent: value.creator,
            status: value.status,
            kind: value.kind,
            created: value.created,
            updated: value.updated,
        }
    }
}

impl From<&showtimes_db::m::Project> for Project {
    fn from(value: &showtimes_db::m::Project) -> Self {
        let poster_url = value.poster.image.as_url();
        Self {
            id: value.id,
            title: value.title.clone(),
            poster_url: Some(poster_url),
            integrations: value.integrations.clone(),
            aliases: value.aliases.clone(),
            parent: value.creator,
            status: value.status,
            kind: value.kind,
            created: value.created,
            updated: value.updated,
        }
    }
}

impl From<&mut showtimes_db::m::Project> for Project {
    fn from(value: &mut showtimes_db::m::Project) -> Self {
        let poster_url = value.poster.image.as_url();
        Self {
            id: value.id,
            title: value.title.clone(),
            poster_url: Some(poster_url),
            integrations: value.integrations.clone(),
            aliases: value.aliases.clone(),
            parent: value.creator,
            status: value.status,
            kind: value.kind,
            created: value.created,
            updated: value.updated,
        }
    }
}

/// The servers model for Showtimes Search data.
///
/// This is mapped from [`showtimes_db::m::Server`] and use `nt-servers` as the index name.
#[derive(Debug, Clone, Serialize, Deserialize, Default, SearchModel)]
#[search(
    name = "nt-servers",
    filterable = ["id", "created", "name", "integrations.id", "integrations.kind"],
    searchable = ["id", "name", "integrations.id"],
    sortable = ["id", "created", "updated"],
    distinct = "id",
)]
pub struct Server {
    /// The unique identifier of the server
    #[serde(with = "ulid_serializer", default = "ulid_serializer::default")]
    #[primary_key]
    pub id: showtimes_shared::ulid::Ulid,
    /// The name of the server
    pub name: String,
    /// The avatar URL of the server
    pub avatar_url: Option<String>,
    /// The integrations of the server
    pub integrations: Vec<showtimes_db::models::IntegrationId>,
    /// The list of owners
    #[serde(with = "ulid_list_serializer")]
    pub owners: Vec<showtimes_shared::ulid::Ulid>,
    /// The date and time the server was created
    #[serde(
        with = "showtimes_shared::unix_timestamp_serializer",
        default = "chrono::Utc::now"
    )]
    pub created: chrono::DateTime<chrono::Utc>,
    /// The date and time the server was last updated
    #[serde(
        with = "showtimes_shared::unix_timestamp_serializer",
        default = "chrono::Utc::now"
    )]
    pub updated: chrono::DateTime<chrono::Utc>,
}

impl From<showtimes_db::m::Server> for Server {
    fn from(value: showtimes_db::m::Server) -> Self {
        let avatar_url = value.avatar.map(|avatar| avatar.as_url());
        let owners_ids = value.owners.iter().map(|owner| owner.id).collect();

        Self {
            id: value.id,
            name: value.name,
            avatar_url,
            owners: owners_ids,
            integrations: value.integrations,
            created: value.created,
            updated: value.updated,
        }
    }
}

/// The user model for Showtimes Search data.
///
/// This is mapped from [`showtimes_db::m::User`] and use `nt-users` as the index name.
#[derive(Debug, Clone, Serialize, Deserialize, Default, SearchModel)]
#[search(
    name = "nt-users",
    filterable = ["id", "created", "username", "discord_id", "discord_username", "api_key.key", "api_key.capabilities", "kind", "registered"],
    searchable = ["id", "username", "discord_id", "discord_username", "api_key"],
    sortable = ["id", "created", "updated"],
    distinct = "id",
)]
pub struct User {
    /// The unique identifier of the user
    #[serde(with = "ulid_serializer", default = "ulid_serializer::default")]
    #[primary_key]
    pub id: showtimes_shared::ulid::Ulid,
    /// The username of the user
    pub username: String,
    /// The avatar URL of the user
    pub avatar_url: Option<String>,
    /// Their ID on Discord
    pub discord_id: String,
    /// Their username on Discord
    pub discord_username: String,
    /// Their API key
    pub api_key: Vec<showtimes_db::m::APIKey>,
    /// Their user kind
    pub kind: showtimes_db::m::UserKind,
    /// Is the user registered or not
    pub registered: bool,
    /// The date and time the user was created
    #[serde(
        with = "showtimes_shared::unix_timestamp_serializer",
        default = "chrono::Utc::now"
    )]
    pub created: chrono::DateTime<chrono::Utc>,
    /// The date and time the user was last updated
    #[serde(
        with = "showtimes_shared::unix_timestamp_serializer",
        default = "chrono::Utc::now"
    )]
    pub updated: chrono::DateTime<chrono::Utc>,
}

impl From<showtimes_db::m::User> for User {
    fn from(value: showtimes_db::m::User) -> Self {
        let avatar_url = value.avatar.map(|avatar| avatar.as_url());
        let discord_id = value.discord_meta.id;
        let discord_username = value.discord_meta.username;

        Self {
            id: value.id,
            username: value.username,
            avatar_url,
            discord_id,
            discord_username,
            api_key: value.api_key,
            kind: value.kind,
            registered: value.registered,
            created: value.created,
            updated: value.updated,
        }
    }
}

/// The information of each collaborator
#[derive(Debug, Clone, Copy, Serialize, Default, Deserialize)]
pub struct ServerCollabTarget {
    /// The server ID
    #[serde(with = "ulid_serializer")]
    pub server: showtimes_shared::ulid::Ulid,
    /// The project ID
    #[serde(with = "ulid_serializer")]
    pub project: showtimes_shared::ulid::Ulid,
}

impl From<showtimes_db::m::ServerCollaborationSyncTarget> for ServerCollabTarget {
    fn from(value: showtimes_db::m::ServerCollaborationSyncTarget) -> Self {
        Self {
            server: value.server,
            project: value.project,
        }
    }
}

impl From<&showtimes_db::m::ServerCollaborationSyncTarget> for ServerCollabTarget {
    fn from(value: &showtimes_db::m::ServerCollaborationSyncTarget) -> Self {
        Self {
            server: value.server,
            project: value.project,
        }
    }
}

/// The server collab model for Showtimes Search data.
///
/// This is mapped from [`showtimes_db::m::ServerCollaborationSync`] and use `nt-collab-sync` as the index name.
#[derive(Debug, Clone, Serialize, Deserialize, Default, SearchModel)]
#[search(
    name = "nt-collab-sync",
    filterable = ["id", "created", "projects.project", "projects.server"],
    searchable = ["id", "projects.project", "projects.server"],
    sortable = ["id", "created", "updated"],
    distinct = "id",
)]
pub struct ServerCollabSync {
    /// The unique identifier of the server
    #[serde(with = "ulid_serializer", default = "ulid_serializer::default")]
    #[primary_key]
    pub id: showtimes_shared::ulid::Ulid,
    /// The project being linked together
    pub projects: Vec<ServerCollabTarget>,
    /// The date and time the server collab was created
    #[serde(
        with = "showtimes_shared::unix_timestamp_serializer",
        default = "chrono::Utc::now"
    )]
    pub created: chrono::DateTime<chrono::Utc>,
    /// The date and time the server collab was last updated
    #[serde(
        with = "showtimes_shared::unix_timestamp_serializer",
        default = "chrono::Utc::now"
    )]
    pub updated: chrono::DateTime<chrono::Utc>,
}

impl From<showtimes_db::m::ServerCollaborationSync> for ServerCollabSync {
    fn from(value: showtimes_db::m::ServerCollaborationSync) -> Self {
        Self {
            id: value.id,
            projects: value
                .projects
                .iter()
                .map(|project| project.into())
                .collect(),
            created: value.created,
            updated: value.updated,
        }
    }
}

/// An information for a collaboration invite (for source)
#[derive(Debug, Clone, Copy, Serialize, Default, Deserialize)]
pub struct ServerCollabInviteSource {
    /// The server ID
    #[serde(with = "ulid_serializer")]
    pub server: showtimes_shared::ulid::Ulid,
    /// The project ID
    #[serde(with = "ulid_serializer")]
    pub project: showtimes_shared::ulid::Ulid,
}

/// An information for a collaboration invite (for target)
#[derive(Debug, Clone, Copy, Serialize, Default, Deserialize)]
pub struct ServerCollabInviteTarget {
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

/// The server collab invite model for Showtimes Search data.
///
/// This is mapped from [`showtimes_db::m::ServerCollaborationInvite`] and use `nt-collab-invite` as the index name.
#[derive(Debug, Clone, Serialize, Deserialize, Default, SearchModel)]
#[search(
    name = "nt-collab-invite",
    filterable = ["id", "source.server", "source.project", "target.server", "target.project", "created"],
    searchable = ["id", "source.server", "source.project", "target.server", "target.project"],
    sortable = ["id", "created", "updated"],
    distinct = "id",
)]
pub struct ServerCollabInvite {
    /// The collab invite ID (unique, and used as invite code too)
    #[serde(with = "ulid_serializer")]
    #[primary_key]
    pub id: showtimes_shared::ulid::Ulid,
    /// The source server
    pub source: ServerCollabInviteSource,
    /// The target server
    pub target: ServerCollabInviteTarget,
    /// The date and time the server collab invite was created
    #[serde(
        with = "showtimes_shared::unix_timestamp_serializer",
        default = "chrono::Utc::now"
    )]
    pub created: chrono::DateTime<chrono::Utc>,
    /// The date and time the server collab invite was last updated
    #[serde(
        with = "showtimes_shared::unix_timestamp_serializer",
        default = "chrono::Utc::now"
    )]
    pub updated: chrono::DateTime<chrono::Utc>,
}

impl From<showtimes_db::m::ServerCollaborationInvite> for ServerCollabInvite {
    fn from(value: showtimes_db::m::ServerCollaborationInvite) -> Self {
        Self {
            id: value.id,
            source: ServerCollabInviteSource {
                server: value.source.server,
                project: value.source.project,
            },
            target: ServerCollabInviteTarget {
                server: value.target.server,
                project: value.target.project,
            },
            created: value.created,
            updated: value.updated,
        }
    }
}
