//! A collection of collaboration events model

use serde::{Deserialize, Serialize};
use showtimes_derive::EventModel;

/// A collab created event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct CollabCreatedEvent {
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
}

impl From<showtimes_db::m::ServerCollaborationInvite> for CollabCreatedEvent {
    fn from(value: showtimes_db::m::ServerCollaborationInvite) -> Self {
        Self { id: value.id }
    }
}

impl From<&showtimes_db::m::ServerCollaborationInvite> for CollabCreatedEvent {
    fn from(value: &showtimes_db::m::ServerCollaborationInvite) -> Self {
        Self { id: value.id }
    }
}

/// A collab accepted event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct CollabAcceptedEvent {
    /// The original invite ID
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
    /// The created sync ID
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    sync_id: showtimes_shared::ulid::Ulid,
}

impl CollabAcceptedEvent {
    /// Create a new collab accepted event
    pub fn new(id: showtimes_shared::ulid::Ulid, sync_id: showtimes_shared::ulid::Ulid) -> Self {
        Self { id, sync_id }
    }
}

/// A collab rejected event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct CollabRejectedEvent {
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
}

impl From<showtimes_db::m::ServerCollaborationInvite> for CollabRejectedEvent {
    fn from(value: showtimes_db::m::ServerCollaborationInvite) -> Self {
        Self { id: value.id }
    }
}

impl From<&showtimes_db::m::ServerCollaborationInvite> for CollabRejectedEvent {
    fn from(value: &showtimes_db::m::ServerCollaborationInvite) -> Self {
        Self { id: value.id }
    }
}

/// A collab retracted event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct CollabRetractedEvent {
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
}

impl CollabRetractedEvent {
    /// Create a new collab retracted event
    pub fn new(id: showtimes_shared::ulid::Ulid) -> Self {
        Self { id }
    }
}

impl From<showtimes_db::m::ServerCollaborationInvite> for CollabRetractedEvent {
    fn from(value: showtimes_db::m::ServerCollaborationInvite) -> Self {
        Self { id: value.id }
    }
}

impl From<&showtimes_db::m::ServerCollaborationInvite> for CollabRetractedEvent {
    fn from(value: &showtimes_db::m::ServerCollaborationInvite) -> Self {
        Self { id: value.id }
    }
}

/// A collab deleted event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct CollabDeletedEvent {
    /// The sync ID
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
    /// The project that detached
    #[event_copy]
    target: showtimes_db::m::ServerCollaborationSyncTarget,
    /// Is the whole collab deleted or just the sync
    #[event_copy]
    is_deleted: bool,
}

impl CollabDeletedEvent {
    /// Create a new collab deleted event
    pub fn new(
        id: showtimes_shared::ulid::Ulid,
        target: &showtimes_db::m::ServerCollaborationSyncTarget,
        is_deleted: bool,
    ) -> Self {
        Self {
            id,
            target: *target,
            is_deleted,
        }
    }
}
