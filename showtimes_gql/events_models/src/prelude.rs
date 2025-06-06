//! A collection of prelude/common types and traits

use async_graphql::{Enum, OutputType, SimpleObject};
use serde::{Serialize, de::DeserializeOwned};
use showtimes_gql_common::{DateTimeGQL, UlidGQL, queries::ServerQueryUser};

use super::{
    collaborations::{
        CollabAcceptedEventDataGQL, CollabCreatedEventDataGQL, CollabDeletedEventDataGQL,
        CollabRejectedEventDataGQL, CollabRetractedEventDataGQL,
    },
    projects::{
        ProjectCreatedEventDataGQL, ProjectDeletedEventDataGQL, ProjectEpisodeUpdatedEventDataGQL,
        ProjectUpdatedEventDataGQL,
    },
    servers::{ServerCreatedEventDataGQL, ServerDeletedEventDataGQL, ServerUpdatedEventDataGQL},
    users::{UserCreatedEventDataGQL, UserDeletedEventDataGQL, UserUpdatedEventDataGQL},
};

/// Implement the [`QueryNew`] trait for a type
pub trait QueryNew<
    O: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + std::fmt::Debug + 'static,
>
{
    /// Create a new event data with specific user request.
    fn new(data: &O, user: ServerQueryUser) -> Self;
}

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
#[graphql(concrete(name = "UserDeletedEventGQL", params(UserDeletedEventDataGQL)))]
#[graphql(concrete(name = "ServerCreatedEventGQL", params(ServerCreatedEventDataGQL)))]
#[graphql(concrete(name = "ServerUpdatedEventGQL", params(ServerUpdatedEventDataGQL)))]
#[graphql(concrete(name = "ServerDeletedEventGQL", params(ServerDeletedEventDataGQL)))]
#[graphql(concrete(name = "ProjectCreatedEventGQL", params(ProjectCreatedEventDataGQL)))]
#[graphql(concrete(name = "ProjectUpdatedEventGQL", params(ProjectUpdatedEventDataGQL)))]
#[graphql(concrete(
    name = "ProjectEpisodeUpdatedEventGQL",
    params(ProjectEpisodeUpdatedEventDataGQL)
))]
#[graphql(concrete(name = "ProjectDeletedEventGQL", params(ProjectDeletedEventDataGQL)))]
#[graphql(concrete(name = "CollabCreatedEventGQL", params(CollabCreatedEventDataGQL)))]
#[graphql(concrete(name = "CollabAcceptedEventGQL", params(CollabAcceptedEventDataGQL)))]
#[graphql(concrete(name = "CollabRejectedEventGQL", params(CollabRejectedEventDataGQL)))]
#[graphql(concrete(name = "CollabRetractedEventGQL", params(CollabRetractedEventDataGQL)))]
#[graphql(concrete(name = "CollabDeletedEventGQL", params(CollabDeletedEventDataGQL)))]
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
    /// Creates a new event with the given data and metadata
    ///
    /// * `id`: The ULID of the event
    /// * `data`: The actual event data that will be broadcasted
    /// * `kind`: The kind of event that is being published
    /// * `actor`: The user that initiated this event, or `None` if it is a system/owner initiated event
    /// * `timestamp`: The timestamp of the event, it is assumed to be in the UTC timezone
    pub fn new(
        id: showtimes_shared::ulid::Ulid,
        data: T,
        kind: EventKindGQL,
        actor: Option<String>,
        timestamp: jiff::Timestamp,
    ) -> Self {
        Self {
            id: UlidGQL::from(id),
            data,
            kind,
            actor,
            timestamp: DateTimeGQL::from(timestamp),
        }
    }
}
