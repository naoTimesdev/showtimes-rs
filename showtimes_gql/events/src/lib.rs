use async_graphql::Object;
use collaborations::{
    CollabAcceptedEventDataGQL, CollabCreatedEventDataGQL, CollabDeletedEventDataGQL,
    CollabRejectedEventDataGQL, CollabRetractedEventDataGQL,
};
use executor::{query_events, query_events_with_user};
use prelude::EventGQL;
use projects::{
    ProjectCreatedEventDataGQL, ProjectDeletedEventDataGQL, ProjectEpisodeUpdatedEventDataGQL,
    ProjectUpdatedEventDataGQL,
};
use servers::{ServerCreatedEventDataGQL, ServerDeletedEventDataGQL, ServerUpdatedEventDataGQL};
use users::{UserCreatedEventDataGQL, UserDeletedEventDataGQL, UserUpdatedEventDataGQL};

mod executor;

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
        query_events_with_user::<showtimes_events::m::UserCreatedEvent, UserCreatedEventDataGQL>(
            ctx,
            id,
            showtimes_events::m::EventKind::UserCreated,
        )
        .await
    }

    /// The user updated event, use `watchUserUpdated` to get a real-time stream instead.
    #[graphql(name = "userUpdated")]
    async fn user_updated(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<UserUpdatedEventDataGQL>>> {
        query_events_with_user::<showtimes_events::m::UserUpdatedEvent, UserUpdatedEventDataGQL>(
            ctx,
            id,
            showtimes_events::m::EventKind::UserUpdated,
        )
        .await
    }

    /// The user deleted event, use `watchUserDeleted` to get a real-time stream instead.
    #[graphql(name = "userDeleted")]
    async fn user_deleted(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<UserDeletedEventDataGQL>>> {
        query_events::<showtimes_events::m::UserDeletedEvent, UserDeletedEventDataGQL>(
            ctx,
            id,
            showtimes_events::m::EventKind::UserDeleted,
        )
        .await
    }

    /// The server created event, use `watchServerCreated` to get a real-time stream instead.
    #[graphql(name = "serverCreated")]
    async fn server_created(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ServerCreatedEventDataGQL>>> {
        query_events::<showtimes_events::m::ServerCreatedEvent, ServerCreatedEventDataGQL>(
            ctx,
            id,
            showtimes_events::m::EventKind::ServerCreated,
        )
        .await
    }

    /// The server updated event, use `watchServerUpdated` to get a real-time stream instead.
    #[graphql(name = "serverUpdated")]
    async fn server_updated(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ServerUpdatedEventDataGQL>>> {
        query_events::<showtimes_events::m::ServerUpdatedEvent, ServerUpdatedEventDataGQL>(
            ctx,
            id,
            showtimes_events::m::EventKind::ServerUpdated,
        )
        .await
    }

    /// The server deleted event, use `watchServerDeleted` to get a real-time stream instead.
    #[graphql(name = "serverDeleted")]
    async fn server_deleted(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ServerDeletedEventDataGQL>>> {
        query_events::<showtimes_events::m::ServerDeletedEvent, ServerDeletedEventDataGQL>(
            ctx,
            id,
            showtimes_events::m::EventKind::ServerDeleted,
        )
        .await
    }

    /// The project created event, use `watchProjectCreated` to get a real-time stream instead.
    #[graphql(name = "projectCreated")]
    async fn project_created(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ProjectCreatedEventDataGQL>>> {
        query_events::<showtimes_events::m::ProjectCreatedEvent, ProjectCreatedEventDataGQL>(
            ctx,
            id,
            showtimes_events::m::EventKind::ProjectCreated,
        )
        .await
    }

    /// The project updated event, use `watchProjectUpdated` to get a real-time stream instead.
    #[graphql(name = "projectUpdated")]
    async fn project_updated(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ProjectUpdatedEventDataGQL>>> {
        query_events::<showtimes_events::m::ProjectUpdatedEvent, ProjectUpdatedEventDataGQL>(
            ctx,
            id,
            showtimes_events::m::EventKind::ProjectUpdated,
        )
        .await
    }

    /// The project episode updated event, use `watchProjectEpisodeUpdated` to get a real-time stream instead.
    #[graphql(name = "projectEpisodeUpdated")]
    async fn project_episode_updated(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ProjectEpisodeUpdatedEventDataGQL>>> {
        query_events::<
            showtimes_events::m::ProjectEpisodeUpdatedEvent,
            ProjectEpisodeUpdatedEventDataGQL,
        >(ctx, id, showtimes_events::m::EventKind::ProjectEpisodes)
        .await
    }

    /// The project deleted event, use `watchProjectDeleted` to get a real-time stream instead.
    #[graphql(name = "projectDeleted")]
    async fn project_deleted(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ProjectDeletedEventDataGQL>>> {
        query_events::<showtimes_events::m::ProjectDeletedEvent, ProjectDeletedEventDataGQL>(
            ctx,
            id,
            showtimes_events::m::EventKind::ProjectDeleted,
        )
        .await
    }

    /// The collaboration created event, use `watchCollabCreated` to get a real-time stream instead.
    #[graphql(name = "collabCreated")]
    async fn collab_created(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<CollabCreatedEventDataGQL>>> {
        query_events::<showtimes_events::m::CollabCreatedEvent, CollabCreatedEventDataGQL>(
            ctx,
            id,
            showtimes_events::m::EventKind::CollaborationCreated,
        )
        .await
    }

    /// The collaboration acceptance event, use `watchCollabAccepted` to get a real-time stream instead.
    #[graphql(name = "collabAccepted")]
    async fn collab_accepted(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<CollabAcceptedEventDataGQL>>> {
        query_events::<showtimes_events::m::CollabAcceptedEvent, CollabAcceptedEventDataGQL>(
            ctx,
            id,
            showtimes_events::m::EventKind::CollaborationAccepted,
        )
        .await
    }

    /// The collaboration rejection event, use `watchCollabRejected` to get a real-time stream instead.
    #[graphql(name = "collabRejected")]
    async fn collab_rejected(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<CollabRejectedEventDataGQL>>> {
        query_events::<showtimes_events::m::CollabRejectedEvent, CollabRejectedEventDataGQL>(
            ctx,
            id,
            showtimes_events::m::EventKind::CollaborationRejected,
        )
        .await
    }

    /// The collaboration retraction event, use `watchCollabRetracted` to get a real-time stream instead.
    #[graphql(name = "collabRetracted")]
    async fn collab_retracted(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<CollabRetractedEventDataGQL>>> {
        query_events::<showtimes_events::m::CollabRetractedEvent, CollabRetractedEventDataGQL>(
            ctx,
            id,
            showtimes_events::m::EventKind::CollaborationRetracted,
        )
        .await
    }

    /// The collaboration deletion or unlinking event, use `watchCollabDeleted` to get a real-time stream instead.
    #[graphql(name = "collabDeleted")]
    async fn collab_deleted(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<CollabDeletedEventDataGQL>>> {
        query_events::<showtimes_events::m::CollabDeletedEvent, CollabDeletedEventDataGQL>(
            ctx,
            id,
            showtimes_events::m::EventKind::CollaborationDeleted,
        )
        .await
    }
}