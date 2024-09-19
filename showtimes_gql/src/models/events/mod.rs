use async_graphql::Object;
use prelude::EventGQL;
use servers::{ServerCreatedEventDataGQL, ServerDeletedEventDataGQL, ServerUpdatedEventDataGQL};
use users::{UserCreatedEventDataGQL, UserDeletedEventDataGQL, UserUpdatedEventDataGQL};

use crate::{data_loader::find_authenticated_user, queries::ServerQueryUser};

pub mod prelude;
pub mod servers;
pub mod users;

/// Search
#[derive(Clone, Copy)]
pub struct QueryEventsRoot;

#[Object]
impl QueryEventsRoot {
    /// The user created event, use `watchUserCreated` to get a real-time stream instead.
    #[graphql(name = "userCreated")]
    async fn user_created(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: crate::models::prelude::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<UserCreatedEventDataGQL>>> {
        let user = find_authenticated_user(ctx).await?;
        let query_stream = ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();

        let mut stream = query_stream
            .query::<showtimes_events::m::UserCreatedEvent>(
                showtimes_events::m::EventKind::UserCreated,
            )
            .start_after(*id);

        let user_query: ServerQueryUser = user.into();

        let mut results = Vec::new();
        while !stream.is_exhausted() {
            let event_batch = stream.advance().await?;
            results.extend(event_batch.into_iter().map(|event| {
                let inner = UserCreatedEventDataGQL::new(event.data(), user_query);
                EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                )
            }));
        }

        Ok(results)
    }

    /// The user updated event, use `watchUserUpdated` to get a real-time stream instead.
    #[graphql(name = "userUpdated")]
    async fn user_updated(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: crate::models::prelude::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<UserUpdatedEventDataGQL>>> {
        let user = find_authenticated_user(ctx).await?;
        let query_stream = ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();

        let mut stream = query_stream
            .query::<showtimes_events::m::UserUpdatedEvent>(
                showtimes_events::m::EventKind::UserUpdated,
            )
            .start_after(*id);

        let user_query: ServerQueryUser = user.into();

        let mut results = Vec::new();
        while !stream.is_exhausted() {
            let event_batch = stream.advance().await?;
            results.extend(event_batch.into_iter().map(|event| {
                let inner = UserUpdatedEventDataGQL::new(event.data(), user_query);
                EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                )
            }));
        }

        Ok(results)
    }

    /// The user deleted event, use `watchUserDeleted` to get a real-time stream instead.
    #[graphql(name = "userDeleted")]
    async fn user_deleted(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: crate::models::prelude::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<UserDeletedEventDataGQL>>> {
        let query_stream = ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();

        let mut stream = query_stream
            .query::<showtimes_events::m::UserDeletedEvent>(
                showtimes_events::m::EventKind::UserDeleted,
            )
            .start_after(*id);

        let mut results = Vec::new();
        while !stream.is_exhausted() {
            let event_batch = stream.advance().await?;
            results.extend(event_batch.into_iter().map(|event| {
                let inner = UserDeletedEventDataGQL::new(event.data());
                EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                )
            }));
        }

        Ok(results)
    }

    /// The server created event, use `watchServerCreated` to get a real-time stream instead.
    #[graphql(name = "serverCreated")]
    async fn server_created(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: crate::models::prelude::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ServerCreatedEventDataGQL>>> {
        let query_stream = ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();

        let mut stream = query_stream
            .query::<showtimes_events::m::ServerCreatedEvent>(
                showtimes_events::m::EventKind::ServerCreated,
            )
            .start_after(*id);

        let mut results = Vec::new();
        while !stream.is_exhausted() {
            let event_batch = stream.advance().await?;
            results.extend(event_batch.into_iter().map(|event| {
                let inner = ServerCreatedEventDataGQL::new(event.data().id());
                EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                )
            }));
        }

        Ok(results)
    }

    /// The server updated event, use `watchServerUpdated` to get a real-time stream instead.
    #[graphql(name = "serverUpdated")]
    async fn server_updated(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: crate::models::prelude::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ServerUpdatedEventDataGQL>>> {
        let query_stream = ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();

        let mut stream = query_stream
            .query::<showtimes_events::m::ServerUpdatedEvent>(
                showtimes_events::m::EventKind::ServerUpdated,
            )
            .start_after(*id);

        let mut results = Vec::new();
        while !stream.is_exhausted() {
            let event_batch = stream.advance().await?;
            results.extend(event_batch.into_iter().map(|event| {
                let inner = ServerUpdatedEventDataGQL::from(event.data());
                EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                )
            }));
        }

        Ok(results)
    }

    /// The server deleted event, use `watchServerDeleted` to get a real-time stream instead.
    #[graphql(name = "serverDeleted")]
    async fn server_deleted(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: crate::models::prelude::UlidGQL,
    ) -> async_graphql::Result<Vec<EventGQL<ServerDeletedEventDataGQL>>> {
        let query_stream = ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();

        let mut stream = query_stream
            .query::<showtimes_events::m::ServerDeletedEvent>(
                showtimes_events::m::EventKind::ServerDeleted,
            )
            .start_after(*id);

        let mut results = Vec::new();
        while !stream.is_exhausted() {
            let event_batch = stream.advance().await?;
            results.extend(event_batch.into_iter().map(|event| {
                let inner = ServerDeletedEventDataGQL::from(event.data());
                EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                )
            }));
        }

        Ok(results)
    }
}
