use async_graphql::{ErrorExtensions, Object};
use collaborations::{
    CollabAcceptedEventDataGQL, CollabCreatedEventDataGQL, CollabDeletedEventDataGQL,
    CollabRejectedEventDataGQL, CollabRetractedEventDataGQL,
};
use prelude::{EventGQL, QueryNew};
use projects::{
    ProjectCreatedEventDataGQL, ProjectDeletedEventDataGQL, ProjectEpisodeUpdatedEventDataGQL,
    ProjectUpdatedEventDataGQL,
};
use servers::{ServerCreatedEventDataGQL, ServerDeletedEventDataGQL, ServerUpdatedEventDataGQL};
use users::{UserCreatedEventDataGQL, UserDeletedEventDataGQL, UserUpdatedEventDataGQL};

use expand::{expand_query_event, expand_query_event_with_user};

mod expand;

pub mod collaborations;
pub mod prelude;
pub mod projects;
pub mod servers;
pub mod users;

/// The root query for events queries.
///
/// This providers multiple queries that can be used to
/// get the stored events log for Showtimes API.
#[derive(Clone, Copy)]
pub struct QueryEventsRoot;

/// The root query for events queries.
///
/// This providers multiple queries that can be used to
/// get the stored events log for Showtimes API.
#[Object]
impl QueryEventsRoot {
    /// The user created event, use `watchUserCreated` to get a real-time stream instead.
    #[graphql(name = "userCreated")]
    async fn user_created(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<UserCreatedEventDataGQL>>> {
        expand_query_event_with_user!(
            ctx,
            id,
            UserCreatedEventDataGQL,
            showtimes_events::m::UserCreatedEvent,
            showtimes_events::m::EventKind::UserCreated
        )
    }

    /// The user updated event, use `watchUserUpdated` to get a real-time stream instead.
    #[graphql(name = "userUpdated")]
    async fn user_updated(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<UserUpdatedEventDataGQL>>> {
        expand_query_event_with_user!(
            ctx,
            id,
            UserUpdatedEventDataGQL,
            showtimes_events::m::UserUpdatedEvent,
            showtimes_events::m::EventKind::UserUpdated
        )
    }

    /// The user deleted event, use `watchUserDeleted` to get a real-time stream instead.
    #[graphql(name = "userDeleted")]
    async fn user_deleted(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<UserDeletedEventDataGQL>>> {
        expand_query_event!(
            ctx,
            id,
            UserDeletedEventDataGQL,
            showtimes_events::m::UserDeletedEvent,
            showtimes_events::m::EventKind::UserDeleted
        )
    }

    /// The server created event, use `watchServerCreated` to get a real-time stream instead.
    #[graphql(name = "serverCreated")]
    async fn server_created(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ServerCreatedEventDataGQL>>> {
        expand_query_event!(
            ctx,
            id,
            ServerCreatedEventDataGQL,
            showtimes_events::m::ServerCreatedEvent,
            showtimes_events::m::EventKind::ServerCreated
        )
    }

    /// The server updated event, use `watchServerUpdated` to get a real-time stream instead.
    #[graphql(name = "serverUpdated")]
    async fn server_updated(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ServerUpdatedEventDataGQL>>> {
        expand_query_event!(
            ctx,
            id,
            ServerUpdatedEventDataGQL,
            showtimes_events::m::ServerUpdatedEvent,
            showtimes_events::m::EventKind::ServerUpdated
        )
    }

    /// The server deleted event, use `watchServerDeleted` to get a real-time stream instead.
    #[graphql(name = "serverDeleted")]
    async fn server_deleted(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ServerDeletedEventDataGQL>>> {
        expand_query_event!(
            ctx,
            id,
            ServerDeletedEventDataGQL,
            showtimes_events::m::ServerDeletedEvent,
            showtimes_events::m::EventKind::ServerDeleted
        )
    }

    /// The project created event, use `watchProjectCreated` to get a real-time stream instead.
    #[graphql(name = "projectCreated")]
    async fn project_created(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ProjectCreatedEventDataGQL>>> {
        expand_query_event!(
            ctx,
            id,
            ProjectCreatedEventDataGQL,
            showtimes_events::m::ProjectCreatedEvent,
            showtimes_events::m::EventKind::ProjectCreated
        )
    }

    /// The project updated event, use `watchProjectUpdated` to get a real-time stream instead.
    #[graphql(name = "projectUpdated")]
    async fn project_updated(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ProjectUpdatedEventDataGQL>>> {
        expand_query_event!(
            ctx,
            id,
            ProjectUpdatedEventDataGQL,
            showtimes_events::m::ProjectUpdatedEvent,
            showtimes_events::m::EventKind::ProjectUpdated
        )
    }

    /// The project episode updated event, use `watchProjectEpisodeUpdated` to get a real-time stream instead.
    #[graphql(name = "projectEpisodeUpdated")]
    async fn project_episode_updated(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ProjectEpisodeUpdatedEventDataGQL>>> {
        expand_query_event!(
            ctx,
            id,
            ProjectEpisodeUpdatedEventDataGQL,
            showtimes_events::m::ProjectEpisodeUpdatedEvent,
            showtimes_events::m::EventKind::ProjectEpisodes
        )
    }

    /// The project deleted event, use `watchProjectDeleted` to get a real-time stream instead.
    #[graphql(name = "projectDeleted")]
    async fn project_deleted(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ProjectDeletedEventDataGQL>>> {
        expand_query_event!(
            ctx,
            id,
            ProjectDeletedEventDataGQL,
            showtimes_events::m::ProjectDeletedEvent,
            showtimes_events::m::EventKind::ProjectDeleted
        )
    }

    /// The collaboration created event, use `watchCollabCreated` to get a real-time stream instead.
    #[graphql(name = "collabCreated")]
    async fn collab_created(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<CollabCreatedEventDataGQL>>> {
        expand_query_event!(
            ctx,
            id,
            CollabCreatedEventDataGQL,
            showtimes_events::m::CollabCreatedEvent,
            showtimes_events::m::EventKind::CollaborationCreated
        )
    }

    /// The collaboration acceptance event, use `watchCollabAccepted` to get a real-time stream instead.
    #[graphql(name = "collabAccepted")]
    async fn collab_accepted(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<CollabAcceptedEventDataGQL>>> {
        expand_query_event!(
            ctx,
            id,
            CollabAcceptedEventDataGQL,
            showtimes_events::m::CollabAcceptedEvent,
            showtimes_events::m::EventKind::CollaborationAccepted
        )
    }

    /// The collaboration rejection event, use `watchCollabRejected` to get a real-time stream instead.
    #[graphql(name = "collabRejected")]
    async fn collab_rejected(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<CollabRejectedEventDataGQL>>> {
        expand_query_event!(
            ctx,
            id,
            CollabRejectedEventDataGQL,
            showtimes_events::m::CollabRejectedEvent,
            showtimes_events::m::EventKind::CollaborationRejected
        )
    }

    /// The collaboration retraction event, use `watchCollabRetracted` to get a real-time stream instead.
    #[graphql(name = "collabRetracted")]
    async fn collab_retracted(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<CollabRetractedEventDataGQL>>> {
        expand_query_event!(
            ctx,
            id,
            CollabRetractedEventDataGQL,
            showtimes_events::m::CollabRetractedEvent,
            showtimes_events::m::EventKind::CollaborationRetracted
        )
    }

    /// The collaboration deletion or unlinking event, use `watchCollabDeleted` to get a real-time stream instead.
    #[graphql(name = "collabDeleted")]
    async fn collab_deleted(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<CollabDeletedEventDataGQL>>> {
        expand_query_event!(
            ctx,
            id,
            CollabDeletedEventDataGQL,
            showtimes_events::m::CollabDeletedEvent,
            showtimes_events::m::EventKind::CollaborationDeleted
        )
    }
}
