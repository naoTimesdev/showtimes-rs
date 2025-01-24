#![warn(missing_docs, clippy::empty_docs, rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../../README.md")]

use std::sync::LazyLock;

use executor::{stream_rss_events, EventWatcher, EventWatcherWithUser};
use futures_util::Stream;

use async_graphql::{Context, Subscription};

use showtimes_gql_common::{guard, queries::ServerQueryUser, UserKindGQL};
use showtimes_gql_events_models::collaborations::{
    CollabAcceptedEventDataGQL, CollabCreatedEventDataGQL, CollabDeletedEventDataGQL,
    CollabRejectedEventDataGQL, CollabRetractedEventDataGQL,
};
use showtimes_gql_events_models::prelude::EventGQL;
use showtimes_gql_events_models::projects::{
    ProjectCreatedEventDataGQL, ProjectDeletedEventDataGQL, ProjectEpisodeUpdatedEventDataGQL,
    ProjectUpdatedEventDataGQL,
};
use showtimes_gql_events_models::rss::RSSEventGQL;
use showtimes_gql_events_models::servers::{
    ServerCreatedEventDataGQL, ServerDeletedEventDataGQL, ServerUpdatedEventDataGQL,
};
use showtimes_gql_events_models::users::{
    UserCreatedEventDataGQL, UserDeletedEventDataGQL, UserUpdatedEventDataGQL,
};

mod executor;

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
        EventWatcherWithUser::<showtimes_events::m::UserCreatedEvent, UserCreatedEventDataGQL>::new(
            showtimes_events::m::EventKind::UserCreated,
            *STUBBED_ADMIN,
        )
        .stream(ctx, id)
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
        EventWatcherWithUser::<showtimes_events::m::UserUpdatedEvent, UserUpdatedEventDataGQL>::new(
            showtimes_events::m::EventKind::UserUpdated,
            *STUBBED_ADMIN,
        )
        .stream(ctx, id)
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
        EventWatcher::<showtimes_events::m::UserDeletedEvent, UserDeletedEventDataGQL>::new(
            showtimes_events::m::EventKind::UserDeleted,
        )
        .stream(ctx, id)
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
        EventWatcher::<showtimes_events::m::ServerCreatedEvent, ServerCreatedEventDataGQL>::new(
            showtimes_events::m::EventKind::ServerCreated,
        )
        .stream(ctx, id)
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
        EventWatcher::<showtimes_events::m::ServerUpdatedEvent, ServerUpdatedEventDataGQL>::new(
            showtimes_events::m::EventKind::ServerUpdated,
        )
        .stream(ctx, id)
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
        EventWatcher::<showtimes_events::m::ServerDeletedEvent, ServerDeletedEventDataGQL>::new(
            showtimes_events::m::EventKind::ServerDeleted,
        )
        .stream(ctx, id)
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
        EventWatcher::<showtimes_events::m::ProjectCreatedEvent, ProjectCreatedEventDataGQL>::new(
            showtimes_events::m::EventKind::ProjectCreated,
        )
        .stream(ctx, id)
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
        EventWatcher::<showtimes_events::m::ProjectUpdatedEvent, ProjectUpdatedEventDataGQL>::new(
            showtimes_events::m::EventKind::ProjectUpdated,
        )
        .stream(ctx, id)
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
        EventWatcher::<
            showtimes_events::m::ProjectEpisodeUpdatedEvent,
            ProjectEpisodeUpdatedEventDataGQL,
        >::new(showtimes_events::m::EventKind::ProjectEpisodes)
        .stream(ctx, id)
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
        EventWatcher::<showtimes_events::m::ProjectDeletedEvent, ProjectDeletedEventDataGQL>::new(
            showtimes_events::m::EventKind::ProjectDeleted,
        )
        .stream(ctx, id)
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
        EventWatcher::<showtimes_events::m::CollabCreatedEvent, CollabCreatedEventDataGQL>::new(
            showtimes_events::m::EventKind::CollaborationCreated,
        )
        .stream(ctx, id)
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
        EventWatcher::<showtimes_events::m::CollabAcceptedEvent, CollabAcceptedEventDataGQL>::new(
            showtimes_events::m::EventKind::CollaborationAccepted,
        )
        .stream(ctx, id)
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
        EventWatcher::<showtimes_events::m::CollabRejectedEvent, CollabRejectedEventDataGQL>::new(
            showtimes_events::m::EventKind::CollaborationRejected,
        )
        .stream(ctx, id)
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
        EventWatcher::<showtimes_events::m::CollabRetractedEvent, CollabRetractedEventDataGQL>::new(
            showtimes_events::m::EventKind::CollaborationRetracted,
        )
        .stream(ctx, id)
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
        EventWatcher::<showtimes_events::m::CollabDeletedEvent, CollabDeletedEventDataGQL>::new(
            showtimes_events::m::EventKind::CollaborationDeleted,
        )
        .stream(ctx, id)
    }

    /// Watch for RSS entry feed event for specific RSS feed
    #[graphql(
        name = "watchRSS",
        guard = "guard::AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn watch_rss_entry(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The RSS feed to query")] feed_id: showtimes_gql_common::UlidGQL,
        #[graphql(desc = "The starting ID to query")] id: Option<showtimes_gql_common::UlidGQL>,
    ) -> impl Stream<Item = RSSEventGQL> {
        stream_rss_events(ctx, feed_id, id)
    }
}
