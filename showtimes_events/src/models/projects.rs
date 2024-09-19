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
