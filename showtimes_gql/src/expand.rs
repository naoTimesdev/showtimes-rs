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

/// Domain expansion: Combined Stream Events Processor
///
/// Merge query stream format and broker stream format into
/// a single request that can be done from any Subscription
#[macro_export]
macro_rules! expand_combined_stream_event {
    ($ctx:expr, $id:expr, $kind:expr, $event:ty, $gql:ty) => {
        let (tx_mem, rx_mem) = tokio::sync::mpsc::channel(100);

        // Spawn the memory broker
        tokio::spawn(async move {
            let mut subscribers = showtimes_events::MemoryBroker::<$event>::subscribe();
            while let Some(event) = subscribers.next().await {
                let inner = <$gql>::from(event.data());
                let parsed_data = $crate::models::events::prelude::EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                );
                if let Err(e) = tx_mem.send(parsed_data).await {
                    // Stop process
                    tracing::warn!("Channel is closed on memory broker, stopping: {e}");
                    break;
                }
            }

            // Close the channel
            drop(tx_mem)
        });

        let mut stream_map = tokio_stream::StreamMap::new();
        stream_map.insert(
            "memory_streams",
            tokio_stream::wrappers::ReceiverStream::new(rx_mem),
        );

        // Process query stream
        if let Some(id) = $id {
            let query_stream = $ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();
            let mut stream = query_stream.query::<$event>($kind).start_after(*id);

            let (tx_query, rx_query) = tokio::sync::mpsc::channel(100);

            // Spawn the query stream
            tokio::spawn(async move {
                while !stream.is_exhausted() {
                    let event_batch = stream.advance().await;
                    match event_batch {
                        Ok(event_batch) => {
                            for event in event_batch.iter() {
                                let inner = <$gql>::from(event.data());
                                let parsed_data = $crate::models::events::prelude::EventGQL::new(
                                    event.id(),
                                    inner,
                                    event.kind().into(),
                                    event.actor().map(|a| a.to_string()),
                                    event.timestamp(),
                                );

                                if let Err(e) = tx_query.send(parsed_data).await {
                                    // Stop process
                                    tracing::warn!(
                                        "Channel is closed on query stream, stopping: {e}"
                                    );
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            // Stop process
                            tracing::warn!("Failed querying data from query stream: {e}");
                            break;
                        }
                    }
                }

                // Close the channel
                drop(tx_query);
            });

            stream_map.insert(
                "query_streams",
                tokio_stream::wrappers::ReceiverStream::new(rx_query),
            );
        }

        return stream_map.map(|(_, item)| item)
    };
    ($ctx:expr, $id:expr, $kind:expr, $event:ty, $gql:ty, $user_stub:expr) => {
        let (tx_mem, rx_mem) = tokio::sync::mpsc::channel(100);

        // Spawn the memory broker
        tokio::spawn(async move {
            let mut subscribers = showtimes_events::MemoryBroker::<$event>::subscribe();
            while let Some(event) = subscribers.next().await {
                let inner = <$gql>::new(event.data(), $user_stub);
                let parsed_data = $crate::models::events::prelude::EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                );
                if let Err(e) = tx_mem.send(parsed_data).await {
                    // Stop process
                    tracing::warn!("Channel is closed on memory broker, stopping: {e}");
                    break;
                }
            }

            // Close the channel
            drop(tx_mem)
        });

        let mut stream_map = tokio_stream::StreamMap::new();
        stream_map.insert(
            "memory_streams",
            tokio_stream::wrappers::ReceiverStream::new(rx_mem),
        );

        // Process query stream
        if let Some(id) = $id {
            let query_stream = $ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();
            let mut stream = query_stream.query::<$event>($kind).start_after(*id);

            let (tx_query, rx_query) = tokio::sync::mpsc::channel(100);

            // Spawn the query stream
            tokio::spawn(async move {
                while !stream.is_exhausted() {
                    let event_batch = stream.advance().await;
                    match event_batch {
                        Ok(event_batch) => {
                            for event in event_batch.iter() {
                                let inner = <$gql>::new(event.data(), $user_stub);
                                let parsed_data = $crate::models::events::prelude::EventGQL::new(
                                    event.id(),
                                    inner,
                                    event.kind().into(),
                                    event.actor().map(|a| a.to_string()),
                                    event.timestamp(),
                                );

                                if let Err(e) = tx_query.send(parsed_data).await {
                                    // Stop process
                                    tracing::warn!(
                                        "Channel is closed on query stream, stopping: {e}"
                                    );
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            // Stop process
                            tracing::warn!("Failed querying data from query stream: {e}");
                            break;
                        }
                    }
                }

                // Close the channel
                drop(tx_query);
            });

            stream_map.insert(
                "query_streams",
                tokio_stream::wrappers::ReceiverStream::new(rx_query),
            );
        }

        return stream_map.map(|(_, item)| item)
    };
}
