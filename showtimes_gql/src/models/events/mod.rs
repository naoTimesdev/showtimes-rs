use async_graphql::Object;
use prelude::EventGQL;
use users::{UserCreatedEventDataGQL, UserUpdatedEventDataGQL};

use crate::{data_loader::find_authenticated_user, queries::ServerQueryUser};

pub mod prelude;
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
}
