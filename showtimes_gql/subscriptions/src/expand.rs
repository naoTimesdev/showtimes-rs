/// Domain expansion: Combined Stream Events Processor
///
/// Merge query stream format and broker stream format into
/// a single request that can be done from any Subscription
macro_rules! expand_combined_stream_event {
    ($ctx:expr, $id:expr, $kind:expr, $event:ty, $gql:ty) => {{
        let (tx_mem, rx_mem) = tokio::sync::mpsc::channel(100);

        // Spawn the memory broker
        tokio::spawn(async move {
            let mut subscribers = showtimes_events::MemoryBroker::<$event>::subscribe();
            while let Some(event) = subscribers.next().await {
                let inner = <$gql>::from(event.data());
                let parsed_data = showtimes_gql_events::prelude::EventGQL::new(
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
                                let parsed_data = showtimes_gql_events::prelude::EventGQL::new(
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

        stream_map.map(|(_, item)| item)
    }};
    ($ctx:expr, $id:expr, $kind:expr, $event:ty, $gql:ty, $user_stub:expr) => {{
        let (tx_mem, rx_mem) = tokio::sync::mpsc::channel(100);

        // Spawn the memory broker
        tokio::spawn(async move {
            let mut subscribers = showtimes_events::MemoryBroker::<$event>::subscribe();
            while let Some(event) = subscribers.next().await {
                let inner = <$gql>::new(event.data(), $user_stub);
                let parsed_data = showtimes_gql_events::prelude::EventGQL::new(
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
                                let parsed_data = showtimes_gql_events::prelude::EventGQL::new(
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

        stream_map.map(|(_, item)| item)
    }};
}

pub(crate) use expand_combined_stream_event;
