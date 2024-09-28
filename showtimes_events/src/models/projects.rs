use serde::{Deserialize, Serialize};
use showtimes_derive::EventModel;

/// A project created event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct ProjectCreatedEvent {
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
    title: String,
}

impl ProjectCreatedEvent {
    pub fn new(id: showtimes_shared::ulid::Ulid, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
        }
    }
}

impl From<showtimes_db::m::Project> for ProjectCreatedEvent {
    fn from(value: showtimes_db::m::Project) -> Self {
        Self {
            id: value.id,
            title: value.title,
        }
    }
}

impl From<&showtimes_db::m::Project> for ProjectCreatedEvent {
    fn from(value: &showtimes_db::m::Project) -> Self {
        Self {
            id: value.id,
            title: value.title.clone(),
        }
    }
}

/// A tiny information about episode update data event
///
/// Used in conjuction with the [`ProjectEpisodeUpdatedEvent`]
#[derive(Debug, Clone, Serialize, Deserialize, Default, EventModel)]
pub struct ProjectUpdatedEpisodeDataEvent {
    /// Unix timestamp of the episode
    aired: Option<i64>,
    /// Episode delay reason
    delay_reason: Option<String>,
}

/// A project updated data event
///
/// Used in conjuction with the [`ProjectUpdatedEvent`]
#[derive(Debug, Clone, Serialize, Deserialize, Default, EventModel)]
pub struct ProjectUpdatedDataEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    integrations: Option<Vec<showtimes_db::m::IntegrationId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    assignees: Option<Vec<showtimes_db::m::RoleAssignee>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    roles: Option<Vec<showtimes_db::m::Role>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    poster_image: Option<showtimes_db::m::ImageMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aliases: Option<Vec<String>>,
    progress: Option<Vec<ProjectUpdatedEpisodeDataEvent>>,
}

/// A project updated event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct ProjectUpdatedEvent {
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
    before: ProjectUpdatedDataEvent,
    after: ProjectUpdatedDataEvent,
}

/// A project episode updated event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct ProjectEpisodeUpdatedEvent {
    /// Project ID
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
    #[event_copy]
    episode: u64,
    #[event_copy]
    finished: Option<bool>,
    before: Vec<showtimes_db::m::RoleStatus>,
    after: Vec<showtimes_db::m::RoleStatus>,
    /// This is silent update, if true, the event should not be broadcasted
    /// when receiving this event, the client should silently update the data
    #[event_copy]
    silent: bool,
}

/// A project deleted event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct ProjectDeletedEvent {
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
}

impl ProjectDeletedEvent {
    pub fn new(id: showtimes_shared::ulid::Ulid) -> Self {
        Self { id }
    }
}

impl From<showtimes_db::m::Project> for ProjectDeletedEvent {
    fn from(value: showtimes_db::m::Project) -> Self {
        Self { id: value.id }
    }
}

impl From<&showtimes_db::m::Project> for ProjectDeletedEvent {
    fn from(value: &showtimes_db::m::Project) -> Self {
        Self { id: value.id }
    }
}
