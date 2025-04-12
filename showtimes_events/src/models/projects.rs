//! A collection of projects events model

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
    /// Create a new [`ProjectCreatedEvent`] with the given `id` and `title`.
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

/// A project updated episode status
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum ProjectUpdatedEpisodeStatus {
    /// New episode added
    New,
    /// Episode removed
    Removed,
    /// Episode updated
    #[default]
    Updated,
}

/// A tiny information about episode update data event
///
/// Used in conjuction with the [`ProjectEpisodeUpdatedEvent`]
#[derive(Debug, Clone, Serialize, Deserialize, Default, EventModel)]
pub struct ProjectUpdatedEpisodeDataEvent {
    /// Episode number in the project
    #[event_copy]
    number: u64,
    /// Unix timestamp of the episode
    #[event_copy]
    aired: Option<i64>,
    /// Episode delay reason
    delay_reason: Option<String>,
    /// Episode status
    #[event_copy]
    status: ProjectUpdatedEpisodeStatus,
}

impl ProjectUpdatedEpisodeDataEvent {
    /// Create a new [`ProjectUpdatedEpisodeDataEvent`] with the given `number` for an addition operation.
    pub fn added(episode: u64) -> Self {
        Self {
            number: episode,
            status: ProjectUpdatedEpisodeStatus::New,
            ..Default::default()
        }
    }

    /// Create a new [`ProjectUpdatedEpisodeDataEvent`] with the given `number` for a removal operation.
    pub fn removed(episode: u64) -> Self {
        Self {
            number: episode,
            status: ProjectUpdatedEpisodeStatus::Removed,
            ..Default::default()
        }
    }

    /// Create a new [`ProjectUpdatedEpisodeDataEvent`] with the given `number` for an update operation.
    pub fn updated(episode: u64) -> Self {
        Self {
            number: episode,
            status: ProjectUpdatedEpisodeStatus::Updated,
            ..Default::default()
        }
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[event_copy]
    status: Option<showtimes_db::m::ProjectStatus>,
    progress: Option<Vec<ProjectUpdatedEpisodeDataEvent>>,
}

impl ProjectUpdatedDataEvent {
    /// Add a new [`ProjectUpdatedEpisodeDataEvent`] to the progress list.
    pub fn add_progress(&mut self, progress: ProjectUpdatedEpisodeDataEvent) {
        match &mut self.progress {
            Some(p) => p.push(progress),
            None => self.progress = Some(vec![progress]),
        }
    }

    /// Check if the event has any changes
    pub fn has_changes(&self) -> bool {
        let has_progress = if let Some(progress) = &self.progress {
            !progress.is_empty()
        } else {
            false
        };

        self.title.is_some()
            || self.integrations.is_some()
            || self.assignees.is_some()
            || self.roles.is_some()
            || self.poster_image.is_some()
            || self.aliases.is_some()
            || self.status.is_some()
            || has_progress
    }
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

impl ProjectUpdatedEvent {
    /// Create a new [`ProjectUpdatedEvent`] with the given `id` and `before` and `after` data.
    pub fn new(
        id: showtimes_shared::ulid::Ulid,
        before: ProjectUpdatedDataEvent,
        after: ProjectUpdatedDataEvent,
    ) -> Self {
        Self { id, before, after }
    }
}

/// A project episode updated event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct ProjectEpisodeUpdatedEvent {
    /// Project ID
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
    #[event_copy]
    number: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[event_copy]
    finished: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    before: Vec<showtimes_db::m::RoleStatus>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    after: Vec<showtimes_db::m::RoleStatus>,
    /// This is silent update, if true, the event should not be broadcasted
    /// when receiving this event, the client should silently update the data
    #[event_copy]
    silent: bool,
}

impl ProjectEpisodeUpdatedEvent {
    /// Create a new [`ProjectEpisodeUpdatedEvent`] with the given `id` and `number`.
    pub fn new(id: showtimes_shared::ulid::Ulid, number: u64, silent: bool) -> Self {
        Self {
            id,
            number,
            finished: None,
            before: Vec::new(),
            after: Vec::new(),
            silent,
        }
    }

    /// Push a previous role status
    pub fn push_before(&mut self, role: &showtimes_db::m::RoleStatus) {
        self.before.push(role.clone());
    }

    /// Push a new role status
    pub fn push_after(&mut self, role: &showtimes_db::m::RoleStatus) {
        self.after.push(role.clone());
    }

    /// Check if the event has any changes
    pub fn has_changes(&self) -> bool {
        !self.before().is_empty() || self.finished().is_some() || !self.after().is_empty()
    }
}

/// A project deleted event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct ProjectDeletedEvent {
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
}

impl ProjectDeletedEvent {
    /// Create a new [`ProjectDeletedEvent`] with the given `id`.
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

impl From<&showtimes_db::m::EpisodeProgress> for ProjectUpdatedEpisodeDataEvent {
    fn from(value: &showtimes_db::m::EpisodeProgress) -> Self {
        Self {
            number: value.number,
            aired: value.aired.map(|v| v.as_second()),
            delay_reason: value.delay_reason.clone(),
            status: ProjectUpdatedEpisodeStatus::Updated,
        }
    }
}
