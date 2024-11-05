/// Domain expansion: Query Events Processor
///
/// Convert internal ClickHouse event types to GraphQL types, then return
/// all the data starting from the provided ID. This macro is designed to
/// helps handle all of that easily since it's the same format for all events.
macro_rules! expand_query_event {
    ($ctx:expr, $id:expr, $gql_type:ty, $event_type:ty, $event_kind:expr) => {{
        let query_stream = $ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();

        let mut stream = query_stream
            .query::<$event_type>($event_kind)
            .start_after(*$id);

        let mut results = Vec::new();
        while !stream.is_exhausted() {
            let event_batch = stream.advance().await.map_err(|err| {
                async_graphql::Error::new(format!(
                    "Failed querying data from query stream: {}",
                    $event_kind.to_name(),
                ))
                .extend_with(|_, e| {
                    e.set("id", $id.to_string());
                    e.set("kind", $event_kind.to_name());
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
                let inner = <$gql_type>::from(event.data());
                $crate::prelude::EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                )
            }));
        }

        Ok(results)
    }};
}

/// Domain expansion: Query Events Processor WITH USER
///
/// Similar to [`expand_query_event`], this version of macro is designed
/// for use if we want to pass a [`crate::ServerQueryUser`] to the GraphQL type.
macro_rules! expand_query_event_with_user {
    ($ctx:expr, $id:expr, $gql_type:ty, $event_type:ty, $event_kind:expr) => {{
        let user = showtimes_gql_common::data_loader::find_authenticated_user($ctx).await?;
        let query_stream = $ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();

        let mut stream = query_stream
            .query::<$event_type>($event_kind)
            .start_after(*$id);

        let user_query: showtimes_gql_common::queries::ServerQueryUser = user.into();

        let mut results = Vec::new();
        while !stream.is_exhausted() {
            let event_batch = stream.advance().await.map_err(|err| {
                async_graphql::Error::new(format!(
                    "Failed querying data from query stream: {}",
                    $event_kind.to_name(),
                ))
                .extend_with(|_, e| {
                    e.set("id", $id.to_string());
                    e.set("kind", $event_kind.to_name());
                    e.set("requester", user_query.id().to_string());
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
                let inner = <$gql_type>::new(event.data(), user_query);
                $crate::prelude::EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                )
            }));
        }

        Ok(results)
    }};
}

pub(crate) use expand_query_event;
pub(crate) use expand_query_event_with_user;
