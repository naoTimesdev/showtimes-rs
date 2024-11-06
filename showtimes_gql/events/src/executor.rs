use async_graphql::{ErrorExtensions, OutputType};
use serde::{de::DeserializeOwned, Serialize};
use showtimes_gql_common::{queries::ServerQueryUser, UlidGQL};

use crate::prelude::{EventGQL, QueryNew};

pub(crate) async fn query_events<O, T>(
    ctx: &async_graphql::Context<'_>,
    id: UlidGQL,
    kind: showtimes_events::m::EventKind,
) -> async_graphql::Result<Vec<EventGQL<T>>>
where
    O: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + std::fmt::Debug + 'static,
    T: for<'target> From<&'target O> + OutputType + 'static,
{
    let query_stream = ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();

    let mut stream = query_stream.query::<O>(kind).start_after(*id);
    let mut results: Vec<EventGQL<T>> = Vec::new();

    while !stream.is_exhausted() {
        let event_batch = stream.advance().await.map_err(|err| {
            async_graphql::Error::new(format!(
                "Failed querying data from query stream: {}",
                kind.to_name(),
            ))
            .extend_with(|_, e| {
                e.set("id", id.to_string());
                e.set("kind", kind.to_name());
                e.set("original", format!("{}", err));
                e.set(
                    "reason",
                    showtimes_gql_common::GQLError::EventAdvanceFailure,
                );
                e.set(
                    "code",
                    showtimes_gql_common::GQLError::EventAdvanceFailure.code(),
                );
            })
        })?;

        results.extend(event_batch.into_iter().map(|event| {
            let inner = T::from(event.data());
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

pub(crate) async fn query_events_with_user<O, T>(
    ctx: &async_graphql::Context<'_>,
    id: UlidGQL,
    kind: showtimes_events::m::EventKind,
) -> async_graphql::Result<Vec<EventGQL<T>>>
where
    O: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + std::fmt::Debug + 'static,
    T: QueryNew<O> + OutputType + 'static,
{
    let user = showtimes_gql_common::data_loader::find_authenticated_user(ctx).await?;
    let query_stream = ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();

    let mut stream = query_stream.query::<O>(kind).start_after(*id);
    let mut results: Vec<EventGQL<T>> = Vec::new();

    let user_query: ServerQueryUser = user.into();

    while !stream.is_exhausted() {
        let event_batch = stream.advance().await.map_err(|err| {
            async_graphql::Error::new(format!(
                "Failed querying data from query stream: {}",
                kind.to_name(),
            ))
            .extend_with(|_, e| {
                e.set("id", id.to_string());
                e.set("kind", kind.to_name());
                e.set("original", format!("{}", err));
                e.set(
                    "reason",
                    showtimes_gql_common::GQLError::EventAdvanceFailure,
                );
                e.set(
                    "code",
                    showtimes_gql_common::GQLError::EventAdvanceFailure.code(),
                );
            })
        })?;

        results.extend(event_batch.into_iter().map(|event| {
            let inner = T::new(event.data(), user_query);
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
