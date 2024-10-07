/// Domain expansion: Query Events Processor
///
/// Convert internal ClickHouse event types to GraphQL types, then return
/// all the data starting from the provided ID. This macro is designed to
/// helps handle all of that easily since it's the same format for all events.
#[macro_export]
macro_rules! expand_query_event {
    ($ctx:expr, $id:expr, $gql_type:ty, $event_type:ty, $event_kind:expr) => {
        let query_stream = $ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();

        let mut stream = query_stream
            .query::<$event_type>($event_kind)
            .start_after(*$id);

        let mut results = Vec::new();
        while !stream.is_exhausted() {
            let event_batch = stream.advance().await?;
            results.extend(event_batch.into_iter().map(|event| {
                let inner = <$gql_type>::from(event.data());
                $crate::models::events::prelude::EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                )
            }));
        }

        return Ok(results);
    };
}

/// Domain expansion: Query Events Processor WITH USER
///
/// Similar to [`expand_query_event`], this version of macro is designed
/// for use if we want to pass a [`crate::ServerQueryUser`] to the GraphQL type.
#[macro_export]
macro_rules! expand_query_event_with_user {
    ($ctx:expr, $id:expr, $gql_type:ty, $event_type:ty, $event_kind:expr) => {
        let user = $crate::data_loader::find_authenticated_user($ctx).await?;
        let query_stream = $ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();

        let mut stream = query_stream
            .query::<$event_type>($event_kind)
            .start_after(*$id);

        let user_query: $crate::queries::ServerQueryUser = user.into();

        let mut results = Vec::new();
        while !stream.is_exhausted() {
            let event_batch = stream.advance().await?;
            results.extend(event_batch.into_iter().map(|event| {
                let inner = <$gql_type>::new(event.data(), user_query);
                $crate::models::events::prelude::EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                )
            }));
        }

        return Ok(results);
    };
}

/// Domain expansion: Stream Events Processor
///
/// Convert internal ClickHouse event types to GraphQL types, then return
/// a stream of all the data from the in-memory broker.
#[macro_export]
macro_rules! expand_stream_event {
    ($kind:ty, $gql:ty) => {
        showtimes_events::MemoryBroker::<$kind>::subscribe().map(move |event| {
            let inner = <$gql>::from(event.data());
            $crate::models::events::prelude::EventGQL::new(
                event.id(),
                inner,
                event.kind().into(),
                event.actor().map(|a| a.to_string()),
                event.timestamp(),
            )
        })
    };

    ($kind:ty, $gql:ty, $stub:expr) => {
        showtimes_events::MemoryBroker::<$kind>::subscribe().map(move |event| {
            let inner = <$gql>::new(event.data(), *$stub);
            $crate::models::events::prelude::EventGQL::new(
                event.id(),
                inner,
                event.kind().into(),
                event.actor().map(|a| a.to_string()),
                event.timestamp(),
            )
        })
    };
}
