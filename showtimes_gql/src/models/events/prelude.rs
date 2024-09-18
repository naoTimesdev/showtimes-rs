use async_graphql::{Enum, OutputType, SimpleObject};

use crate::models::prelude::*;

use super::users::{UserCreatedEventDataGQL, UserUpdatedEventDataGQL};

/// Represents the kind of event that can be published
#[derive(Enum, Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[graphql(
    remote = "showtimes_events::m::EventKind",
    rename_items = "SCREAMING_SNAKE_CASE"
)]
pub enum EventKindGQL {
    /// User created event
    UserCreated = 1,
    /// User updated event
    UserUpdated = 2,
    /// User deleted event
    UserDeleted = 3,
    /// Server created event
    ServerCreated = 10,
    /// Server updated event
    ServerUpdated = 11,
    /// Server deleted event
    ServerDeleted = 12,
    /// Project created event
    ProjectCreated = 20,
    /// Project updated event
    ProjectUpdated = 21,
    /// Project deleted event
    ProjectDeleted = 22,
    /// Project episodes event, this is used to publish information
    /// changes for a single episode of a project.
    ProjectEpisodes = 30,
    /// Collaboration created event
    CollaborationCreated = 40,
    /// Collaboration accepted event
    CollaborationAccepted = 41,
    /// Collaboration rejected event
    CollaborationRejected = 42,
    /// Collaboration deleted event
    CollaborationDeleted = 43,
    /// Collaboration retracted event, used when the initiator cancels
    CollaborationRetracted = 44,
}

/// The event structure that is broadcasted and stored
#[derive(SimpleObject)]
#[graphql(concrete(name = "UserCreatedEventGQL", params(UserCreatedEventDataGQL)))]
#[graphql(concrete(name = "UserUpdatedEventGQL", params(UserUpdatedEventDataGQL)))]
pub struct EventGQL<T: OutputType> {
    /// The event ID
    id: UlidGQL,
    /// The event data information
    data: T,
    /// The event kind information
    kind: EventKindGQL,
    /// The actor or the person who initiated the event
    ///
    /// This is an ULID compatible string
    ///
    /// If the event is initiated by the system/Owner, this will be `null`
    actor: Option<String>,
    /// The timestamp of the event
    timestamp: DateTimeGQL,
}

impl<T> EventGQL<T>
where
    T: OutputType,
{
    pub(crate) fn new(
        id: showtimes_shared::ulid::Ulid,
        data: T,
        kind: EventKindGQL,
        actor: Option<String>,
        timestamp: ::time::OffsetDateTime,
    ) -> Self {
        // Convert the timestamp to a chrono date time
        let ts_unix = timestamp.unix_timestamp();
        let chrono_ts = chrono::DateTime::<chrono::Utc>::from_timestamp(ts_unix, 0).unwrap();

        Self {
            id: UlidGQL::from(id),
            data,
            kind,
            actor,
            timestamp: DateTimeGQL::from(chrono_ts),
        }
    }
}
