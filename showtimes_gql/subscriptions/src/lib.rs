#![warn(missing_docs, clippy::empty_docs, rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../../README.md")]

use std::sync::LazyLock;

use futures_util::{Stream, StreamExt};

use async_graphql::{Context, Subscription};

use showtimes_gql_common::{guard, queries::ServerQueryUser, UserKindGQL};
use showtimes_gql_events::collaborations::{
    CollabAcceptedEventDataGQL, CollabCreatedEventDataGQL, CollabDeletedEventDataGQL,
    CollabRejectedEventDataGQL, CollabRetractedEventDataGQL,
};
use showtimes_gql_events::prelude::EventGQL;
use showtimes_gql_events::projects::{
    ProjectCreatedEventDataGQL, ProjectDeletedEventDataGQL, ProjectEpisodeUpdatedEventDataGQL,
    ProjectUpdatedEventDataGQL,
};
use showtimes_gql_events::servers::{
    ServerCreatedEventDataGQL, ServerDeletedEventDataGQL, ServerUpdatedEventDataGQL,
};
use showtimes_gql_events::users::{
    UserCreatedEventDataGQL, UserDeletedEventDataGQL, UserUpdatedEventDataGQL,
};

mod expand;
pub(crate) use expand::expand_combined_stream_event;

static STUBBED_ADMIN: LazyLock<ServerQueryUser> = LazyLock::new(|| {
    ServerQueryUser::new(
        showtimes_shared::ulid::Ulid::new(),
        showtimes_db::m::UserKind::Admin,
    )
});

/// The main Subscription Root type for the GraphQL schema. This is where all the subscription are defined.
pub struct SubscriptionRoot;

/// The main Subscription Root type for the GraphQL schema. This is where all the subscription are defined.
#[Subscription]
impl SubscriptionRoot {
    /// Watch for user created events
    #[graphql(
        name = "watchUserCreated",
        guard = "guard::AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn watch_user_created(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<showtimes_gql_common::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<UserCreatedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::UserCreated,
            showtimes_events::m::UserCreatedEvent,
            UserCreatedEventDataGQL,
            *STUBBED_ADMIN
        )
    }

    /// Watch for user updates events
    #[graphql(
        name = "watchUserUpdated",
        guard = "guard::AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn watch_user_updated(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<showtimes_gql_common::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<UserUpdatedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::UserUpdated,
            showtimes_events::m::UserUpdatedEvent,
            UserUpdatedEventDataGQL,
            *STUBBED_ADMIN
        )
    }

    /// Watch for user deleted events
    #[graphql(
        name = "watchUserDeleted",
        guard = "guard::AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn watch_user_deleted(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<showtimes_gql_common::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<UserDeletedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::UserDeleted,
            showtimes_events::m::UserDeletedEvent,
            UserDeletedEventDataGQL
        )
    }

    /// Watch for server created events
    #[graphql(
        name = "watchServerCreated",
        guard = "guard::AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn watch_server_created(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<showtimes_gql_common::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<ServerCreatedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::ServerCreated,
            showtimes_events::m::ServerCreatedEvent,
            ServerCreatedEventDataGQL
        )
    }

    /// Watch for server updates events
    #[graphql(
        name = "watchServerUpdated",
        guard = "guard::AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn watch_server_updated(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<showtimes_gql_common::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<ServerUpdatedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::ServerUpdated,
            showtimes_events::m::ServerUpdatedEvent,
            ServerUpdatedEventDataGQL
        )
    }

    /// Watch for server deleted events
    #[graphql(
        name = "watchServerDeleted",
        guard = "guard::AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn watch_server_deleted(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<showtimes_gql_common::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<ServerDeletedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::ServerDeleted,
            showtimes_events::m::ServerDeletedEvent,
            ServerDeletedEventDataGQL
        )
    }

    /// Watch for project created events
    #[graphql(
        name = "watchProjectCreated",
        guard = "guard::AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn watch_project_created(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<showtimes_gql_common::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<ProjectCreatedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::ProjectCreated,
            showtimes_events::m::ProjectCreatedEvent,
            ProjectCreatedEventDataGQL
        )
    }

    /// Watch for project updates events
    #[graphql(
        name = "watchProjectUpdated",
        guard = "guard::AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn watch_project_updated(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<showtimes_gql_common::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<ProjectUpdatedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::ProjectUpdated,
            showtimes_events::m::ProjectUpdatedEvent,
            ProjectUpdatedEventDataGQL
        )
    }

    /// Watch for project episodes update events
    #[graphql(
        name = "watchProjectEpisodeUpdated",
        guard = "guard::AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn watch_project_episode_updated(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<showtimes_gql_common::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<ProjectEpisodeUpdatedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::ProjectEpisodes,
            showtimes_events::m::ProjectEpisodeUpdatedEvent,
            ProjectEpisodeUpdatedEventDataGQL
        )
    }

    /// Watch for project deleted events
    #[graphql(
        name = "watchProjectDeleted",
        guard = "guard::AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn watch_project_deleted(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<showtimes_gql_common::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<ProjectDeletedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::ProjectDeleted,
            showtimes_events::m::ProjectDeletedEvent,
            ProjectDeletedEventDataGQL
        )
    }

    /// Watch for collaboration created events
    #[graphql(
        name = "watchCollabCreated",
        guard = "guard::AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn watch_collab_created(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<showtimes_gql_common::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<CollabCreatedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::CollaborationCreated,
            showtimes_events::m::CollabCreatedEvent,
            CollabCreatedEventDataGQL
        )
    }

    /// Watch for collaboration acceptances events
    #[graphql(
        name = "watchCollabAccepted",
        guard = "guard::AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn watch_collab_accepted(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<showtimes_gql_common::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<CollabAcceptedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::CollaborationAccepted,
            showtimes_events::m::CollabAcceptedEvent,
            CollabAcceptedEventDataGQL
        )
    }

    /// Watch for collaboration rejection events
    #[graphql(
        name = "watchCollabRejected",
        guard = "guard::AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn watch_collab_rejected(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<showtimes_gql_common::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<CollabRejectedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::CollaborationRejected,
            showtimes_events::m::CollabRejectedEvent,
            CollabRejectedEventDataGQL
        )
    }

    /// Watch for collaboration retraction events
    #[graphql(
        name = "watchCollabRetracted",
        guard = "guard::AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn watch_collab_retracted(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<showtimes_gql_common::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<CollabRetractedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::CollaborationRetracted,
            showtimes_events::m::CollabRetractedEvent,
            CollabRetractedEventDataGQL
        )
    }

    /// Watch for collaboration deleted events
    #[graphql(
        name = "watchCollabDeleted",
        guard = "guard::AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn watch_collab_deleted(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<showtimes_gql_common::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<CollabDeletedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::CollaborationDeleted,
            showtimes_events::m::CollabDeletedEvent,
            CollabDeletedEventDataGQL
        )
    }
}
