use std::{fmt::Debug, marker::PhantomData};

use async_graphql::OutputType;
use futures_util::{Stream, StreamExt};
use serde::{Serialize, de::DeserializeOwned};
use showtimes_gql_common::{UlidGQL, queries::ServerQueryUser};
use showtimes_gql_events_models::{
    prelude::{EventGQL, QueryNew},
    rss::RSSEventGQL,
};

pub(crate) struct EventWatcher<
    O: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + Debug + 'static,
    T: for<'target> From<&'target O> + OutputType + 'static,
> {
    kind: showtimes_events::m::EventKind,
    _pin_o: PhantomData<O>,
    _pin_t: PhantomData<T>,
}

pub(crate) struct EventWatcherWithUser<
    O: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + Debug + 'static,
    T: QueryNew<O> + OutputType + 'static,
> {
    kind: showtimes_events::m::EventKind,
    user: ServerQueryUser,
    _pin_o: PhantomData<O>,
    _pin_t: PhantomData<T>,
}

impl<
    O: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + Debug + 'static,
    T: for<'target> From<&'target O> + OutputType + 'static,
> EventWatcher<O, T>
{
    pub(crate) fn new(kind: showtimes_events::m::EventKind) -> Self {
        Self {
            kind,
            _pin_o: PhantomData,
            _pin_t: PhantomData,
        }
    }

    pub(crate) fn stream(
        self,
        ctx: &async_graphql::Context<'_>,
        id: Option<UlidGQL>,
    ) -> impl Stream<Item = EventGQL<T>> + use<O, T> {
        let mut stream_map = tokio_stream::StreamMap::new();
        let (tx_mem, rx_mem) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            let mut subscribers = showtimes_events::MemoryBroker::<O>::subscribe();
            while let Some(event) = subscribers.next().await {
                let inner = T::from(event.data());
                let parsed_data = EventGQL::new(
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

        stream_map.insert(
            "memory_stream",
            tokio_stream::wrappers::ReceiverStream::new(rx_mem),
        );

        if let Some(id) = id {
            let query_stream = ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();
            let mut stream = query_stream.query::<O>(self.kind).start_after(*id);

            let (tx_query, rx_query) = tokio::sync::mpsc::channel(100);
            tokio::spawn(async move {
                while !stream.is_exhausted() {
                    let event_batch = stream.advance().await;
                    match event_batch {
                        Ok(event_batch) => {
                            for event in event_batch.iter() {
                                let inner = T::from(event.data());
                                let parsed_data = EventGQL::new(
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
            });

            stream_map.insert(
                "query_streams",
                tokio_stream::wrappers::ReceiverStream::new(rx_query),
            );
        };

        stream_map.map(|(_, item)| item)
    }
}

impl<
    O: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + Debug + 'static,
    T: QueryNew<O> + OutputType + 'static,
> EventWatcherWithUser<O, T>
{
    pub(crate) fn new(kind: showtimes_events::m::EventKind, user: ServerQueryUser) -> Self {
        Self {
            kind,
            user,
            _pin_o: PhantomData,
            _pin_t: PhantomData,
        }
    }

    pub(crate) fn stream(
        self,
        ctx: &async_graphql::Context<'_>,
        id: Option<UlidGQL>,
    ) -> impl Stream<Item = EventGQL<T>> + use<O, T> {
        let mut stream_map = tokio_stream::StreamMap::new();
        let (tx_mem, rx_mem) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            let mut subscribers = showtimes_events::MemoryBroker::<O>::subscribe();
            while let Some(event) = subscribers.next().await {
                let inner = T::new(event.data(), self.user);
                let parsed_data = EventGQL::new(
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

        stream_map.insert(
            "memory_stream",
            tokio_stream::wrappers::ReceiverStream::new(rx_mem),
        );

        if let Some(id) = id {
            let query_stream = ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();
            let mut stream = query_stream.query::<O>(self.kind).start_after(*id);

            let (tx_query, rx_query) = tokio::sync::mpsc::channel(100);
            tokio::spawn(async move {
                while !stream.is_exhausted() {
                    let event_batch = stream.advance().await;
                    match event_batch {
                        Ok(event_batch) => {
                            for event in event_batch.iter() {
                                let inner = T::new(event.data(), self.user);
                                let parsed_data = EventGQL::new(
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
            });

            stream_map.insert(
                "query_streams",
                tokio_stream::wrappers::ReceiverStream::new(rx_query),
            );
        };

        stream_map.map(|(_, item)| item)
    }
}

pub(crate) fn stream_rss_events(
    ctx: &async_graphql::Context<'_>,
    feed_id: UlidGQL,
    id: Option<UlidGQL>,
) -> impl Stream<Item = RSSEventGQL> + use<> {
    let mut stream_map = tokio_stream::StreamMap::new();
    let (tx_mem, rx_mem) = tokio::sync::mpsc::channel(100);

    tokio::spawn(async move {
        let mut subscribers = showtimes_events::RSSBroker::subscribe(*feed_id);
        while let Some(event) = subscribers.next().await {
            let parsed_data = RSSEventGQL::from(event);
            if let Err(e) = tx_mem.send(parsed_data).await {
                // Stop process
                tracing::warn!("Channel is closed on RSS broker, stopping: {e}");
                break;
            }
        }

        // Close the channel
        drop(tx_mem)
    });

    stream_map.insert(
        "memory_stream",
        tokio_stream::wrappers::ReceiverStream::new(rx_mem),
    );

    if let Some(id) = id {
        let query_stream = ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();
        let mut stream = query_stream.query_rss(*feed_id).start_after(*id);

        let (tx_query, rx_query) = tokio::sync::mpsc::channel(100);
        tokio::spawn(async move {
            while !stream.is_exhausted() {
                let event_batch = stream.advance().await;
                match event_batch {
                    Ok(event_batch) => {
                        for event in event_batch.iter() {
                            let parsed_data = RSSEventGQL::from(event);

                            if let Err(e) = tx_query.send(parsed_data).await {
                                // Stop process
                                tracing::warn!(
                                    "Channel is closed on RSS query stream, stopping: {e}"
                                );
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        // Stop process
                        tracing::warn!("Failed querying data from RSS query stream: {e}");
                        break;
                    }
                }
            }
        });

        stream_map.insert(
            "query_streams",
            tokio_stream::wrappers::ReceiverStream::new(rx_query),
        );
    };

    stream_map.map(|(_, item)| item)
}
