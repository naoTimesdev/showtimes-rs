//! A collection of event executor functions for querying events from ClickHouse

use async_graphql::OutputType;
use serde::{de::DeserializeOwned, Serialize};
use showtimes_gql_common::{errors::GQLError, queries::ServerQueryUser, GQLErrorCode, UlidGQL};

use showtimes_gql_events_models::{
    prelude::{EventGQL, QueryNew},
    rss::RSSEventGQL,
};

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
            GQLError::new(
                format!("Failed querying data from query stream: {}", kind.to_name(),),
                GQLErrorCode::EventAdvanceFailure,
            )
            .extend(|e| {
                e.set("id", id.to_string());
                e.set("kind", kind.to_name());
                e.set("original", format!("{}", err));
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
    let user = ctx.data_unchecked::<showtimes_db::m::User>();
    let query_stream = ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();

    let mut stream = query_stream.query::<O>(kind).start_after(*id);
    let mut results: Vec<EventGQL<T>> = Vec::new();

    let user_query: ServerQueryUser = user.into();

    while !stream.is_exhausted() {
        let event_batch = stream.advance().await.map_err(|err| {
            GQLError::new(
                format!("Failed querying data from query stream: {}", kind.to_name(),),
                GQLErrorCode::EventAdvanceFailure,
            )
            .extend(|e| {
                e.set("id", id.to_string());
                e.set("kind", kind.to_name());
                e.set("original", format!("{}", err));
                e.set("user", user_query.id().to_string());
                e.set("user_kind", user_query.kind().to_name());
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

pub(crate) async fn query_rss_events(
    ctx: &async_graphql::Context<'_>,
    feed_id: UlidGQL,
    id: UlidGQL,
) -> async_graphql::Result<Vec<RSSEventGQL>> {
    let query_stream = ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();

    let mut stream = query_stream.query_rss(*feed_id).start_after(*id);
    let mut results: Vec<RSSEventGQL> = Vec::new();

    while !stream.is_exhausted() {
        let event_batch = stream.advance().await.map_err(|err| {
            GQLError::new(
                format!(
                    "Failed querying data from RSS query stream: {}",
                    feed_id.to_string()
                ),
                GQLErrorCode::EventRSSAdvanceFailure,
            )
            .extend(|e| {
                e.set("id", id.to_string());
                e.set("feed_id", feed_id.to_string());
                e.set("original", format!("{}", err));
            })
        })?;

        results.extend(event_batch.into_iter().map(RSSEventGQL::from));
    }

    Ok(results)
}
