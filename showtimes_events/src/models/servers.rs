//! A collection of servers events model

use serde::{Deserialize, Serialize};
use showtimes_derive::EventModel;

/// A server created event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct ServerCreatedEvent {
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
    name: String,
}

impl From<showtimes_db::m::Server> for ServerCreatedEvent {
    fn from(value: showtimes_db::m::Server) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<&showtimes_db::m::Server> for ServerCreatedEvent {
    fn from(value: &showtimes_db::m::Server) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
        }
    }
}

/// A server updated data event
///
/// Used in conjuction with the [`ServerUpdatedEvent`]
#[derive(Debug, Clone, Serialize, Deserialize, Default, EventModel)]
pub struct ServerUpdatedDataEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    integrations: Option<Vec<showtimes_db::m::IntegrationId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owners: Option<Vec<showtimes_db::m::ServerUser>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar: Option<showtimes_db::m::ImageMetadata>,
}

/// A server updated event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct ServerUpdatedEvent {
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
    before: ServerUpdatedDataEvent,
    after: ServerUpdatedDataEvent,
}

impl ServerUpdatedEvent {
    /// Creates a new server updated event
    pub fn new(
        id: showtimes_shared::ulid::Ulid,
        before: ServerUpdatedDataEvent,
        after: ServerUpdatedDataEvent,
    ) -> Self {
        Self { id, before, after }
    }
}

/// A server deleted event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct ServerDeletedEvent {
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
}

impl ServerDeletedEvent {
    /// Creates a new server deleted event
    pub fn new(id: showtimes_shared::ulid::Ulid) -> Self {
        Self { id }
    }
}

impl From<showtimes_db::m::Server> for ServerDeletedEvent {
    fn from(value: showtimes_db::m::Server) -> Self {
        Self { id: value.id }
    }
}

impl From<&showtimes_db::m::Server> for ServerDeletedEvent {
    fn from(value: &showtimes_db::m::Server) -> Self {
        Self { id: value.id }
    }
}
