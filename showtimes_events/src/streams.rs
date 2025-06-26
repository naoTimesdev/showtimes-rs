use std::fmt::Debug;

use clickhouse::Client;

use crate::{
    RSS_TABLE_NAME, TABLE_NAME,
    m::EventKind,
    models::{RSSEvent, SHEvent},
};

#[derive(Clone)]
pub struct SHClickStream<
    T: serde::de::DeserializeOwned + Send + Sync + Clone + Unpin + Debug + 'static,
> {
    client: Client,
    kind: EventKind,
    // User defined
    start_after: Option<uuid::Uuid>,
    // Our internal state
    internal_offset: Option<usize>,
    per_page: usize,
    initialize: bool,
    upper_bound: Option<usize>,
    current: Option<Vec<SHEvent<T>>>,
}

impl<T> SHClickStream<T>
where
    T: serde::de::DeserializeOwned + Send + Sync + Clone + Unpin + Debug + 'static,
{
    pub(crate) fn init(client: Client, kind: EventKind) -> Self {
        tracing::debug!("Initializing SHClickStream for kind {:?}", kind);
        Self {
            client,
            kind,
            start_after: None,
            internal_offset: None,
            current: None,
            initialize: false,
            upper_bound: None,
            per_page: 50,
        }
    }

    /// Set the per page count
    pub fn per_page(mut self, per_page: usize) -> Self {
        if self.initialize {
            // Ignore if already initialized
            return self;
        }

        self.per_page = per_page;
        self
    }

    /// Start after a specific ULID
    pub fn start_after(mut self, start_after: showtimes_shared::ulid::Ulid) -> Self {
        if self.initialize {
            // Ignore if already initialized
            return self;
        }

        self.start_after = Some(showtimes_shared::ulid_to_uuid(start_after));
        self
    }

    pub async fn current(&self) -> Option<Vec<SHEvent<T>>> {
        self.current.clone()
    }

    async fn calculate(&mut self) -> Result<(), clickhouse::error::Error> {
        if self.upper_bound.is_some() {
            return Ok(());
        }

        #[derive(Debug, serde::Deserialize, clickhouse::Row)]
        struct InternalCounter {
            upper_bound: u64,
        }

        let result_await = match self.start_after {
            Some(start_after) => self
                .client
                .query(&format!(
                    r#"SELECT count() AS ?fields FROM {TABLE_NAME}
                    WHERE (
                        toUInt128(id) > toUInt128(toUUID(?)) AND
                        kind = ?
                    )
                    "#,
                ))
                .bind(start_after.to_string())
                .bind(self.kind as u8)
                .fetch_one::<InternalCounter>(),
            None => self
                .client
                .query(&format!(
                    r#"SELECT count() AS ?fields FROM {TABLE_NAME}
                    WHERE (
                        kind = ?
                    )
                    "#,
                ))
                .bind(self.kind as u8)
                .fetch_one::<InternalCounter>(),
        };

        tracing::debug!("Starting upper bound query for: {:?}", self.kind);
        let count = result_await.await?;

        tracing::debug!("Result upper bound query for: {:?}", count);
        self.upper_bound = Some(count.upper_bound as usize);

        Ok(())
    }

    pub async fn advance(&mut self) -> Result<Vec<SHEvent<T>>, clickhouse::error::Error> {
        // Do initial count
        self.calculate().await?;

        let offset = self.internal_offset.unwrap_or(0);

        tracing::debug!(
            "Requesting SHClickStream for kind {:?} with offset {}",
            self.kind,
            offset
        );
        let all_events = match self.start_after {
            Some(start_after) => {
                self.client
                    .query(&format!(
                        r#"SELECT ?fields FROM {TABLE_NAME}
                           WHERE (
                               toUInt128(id) > toUInt128(toUUID(?)) AND
                               kind = ?
                           )
                           ORDER BY toUInt128(id) ASC
                           OFFSET ? ROW FETCH FIRST ? ROWS ONLY"#,
                    ))
                    .bind(start_after.to_string())
                    .bind(self.kind as u8)
                    .bind(offset)
                    .bind(self.per_page)
                    .fetch_all::<SHEvent<T>>()
                    .await?
            }
            None => {
                self.client
                    .query(&format!(
                        r#"SELECT ?fields FROM {TABLE_NAME}
                           WHERE (
                               kind = ?
                           )
                           ORDER BY toUInt128(id) ASC
                           OFFSET ? ROW FETCH FIRST ? ROWS ONLY"#,
                    ))
                    .bind(self.kind as u8)
                    .bind(offset)
                    .bind(self.per_page)
                    .fetch_all::<SHEvent<T>>()
                    .await?
            }
        };

        tracing::debug!(
            "Got {} SHClickStream for kind {:?} with offset {}",
            all_events.len(),
            self.kind,
            offset
        );

        self.initialize = true;
        self.internal_offset = Some(offset + self.per_page);
        self.current = Some(all_events.clone());

        Ok(all_events)
    }

    // Check if exhausted
    pub fn is_exhausted(&self) -> bool {
        if self.initialize && self.current.is_none() {
            return true;
        }

        match &self.current {
            Some(current) => current.len() < self.per_page,
            _ => false,
        }
    }

    /// Fetch all events
    pub async fn fetch_all(&mut self) -> Result<Vec<SHEvent<T>>, clickhouse::error::Error> {
        let mut all_events = Vec::new();
        loop {
            let events = self.advance().await?;
            if events.is_empty() || self.is_exhausted() {
                break;
            }

            all_events.extend(events);
        }

        Ok(all_events)
    }
}

// impl<T> futures_util::TryStream for SHClickStream<T>
// where
//     T: serde::de::DeserializeOwned + Send + Sync + Clone + Debug + Unpin + 'static,
// {
//     type Ok = Vec<SHEvent<T>>;
//     type Error = clickhouse::error::Error;

//     fn try_poll_next(
//         mut self: std::pin::Pin<&mut Self>,
//         _: &mut std::task::Context,
//     ) -> std::task::Poll<Option<Result<Self::Ok, Self::Error>>> {
//         tracing::debug!("Polling SHClickStream for kind {:?}", self.kind);
//         if self.is_exhausted() {
//             tracing::debug!("Exhausted SHClickStream for kind {:?}", self.kind);
//             return std::task::Poll::Ready(None);
//         }

//         let fut = self.advance();
//         tracing::debug!("Polling SHClickStream for with future");
//         let res = fut.now_or_never();
//         tracing::debug!("Polling SHClickStream for with future result: {:?}", &res);
//         match res {
//             Some(Ok(res)) => {
//                 // If we have empty result, we should return None
//                 if res.is_empty() {
//                     std::task::Poll::Ready(None)
//                 } else {
//                     std::task::Poll::Ready(Some(Ok(res)))
//                 }
//             }
//             Some(Err(e)) => {
//                 tracing::error!(
//                     "Error polling SHClickStream for kind {:?}: {:?}",
//                     self.kind,
//                     e
//                 );
//                 std::task::Poll::Ready(Some(Err(e)))
//             }
//             None => std::task::Poll::Pending,
//         }
//     }
// }

#[derive(Clone)]
pub struct SHRSSClickStream {
    client: Client,
    feed_id: uuid::Uuid,
    // User defined
    start_after: Option<uuid::Uuid>,
    // Our internal state
    internal_offset: Option<usize>,
    per_page: usize,
    initialize: bool,
    upper_bound: Option<usize>,
    current: Option<Vec<RSSEvent>>,
}

impl SHRSSClickStream {
    pub(crate) fn init(client: Client, feed_id: showtimes_shared::ulid::Ulid) -> Self {
        tracing::debug!("Initializing SHRSSClickStream for feed {:?}", feed_id);
        Self {
            client,
            feed_id: showtimes_shared::ulid_to_uuid(feed_id),
            start_after: None,
            internal_offset: None,
            current: None,
            initialize: false,
            upper_bound: None,
            per_page: 50,
        }
    }

    /// Set the per page count
    pub fn per_page(mut self, per_page: usize) -> Self {
        if self.initialize {
            // Ignore if already initialized
            return self;
        }

        self.per_page = per_page;
        self
    }

    /// Start after a specific ULID
    pub fn start_after(mut self, start_after: showtimes_shared::ulid::Ulid) -> Self {
        if self.initialize {
            // Ignore if already initialized
            return self;
        }

        self.start_after = Some(showtimes_shared::ulid_to_uuid(start_after));
        self
    }

    pub async fn current(&self) -> Option<Vec<RSSEvent>> {
        self.current.clone()
    }

    async fn calculate(&mut self) -> Result<(), clickhouse::error::Error> {
        if self.upper_bound.is_some() {
            return Ok(());
        }

        #[derive(Debug, serde::Deserialize, clickhouse::Row)]
        struct InternalCounter {
            upper_bound: u64,
        }

        let result_await = match self.start_after {
            Some(start_after) => self
                .client
                .query(&format!(
                    r#"SELECT count() AS ?fields FROM {RSS_TABLE_NAME}
                    WHERE (
                        toUInt128(id) > toUInt128(toUUID(?)) AND
                        feed_id = toUUID(?)
                    )
                    "#,
                ))
                .bind(start_after.to_string())
                .bind(self.feed_id.to_string())
                .fetch_one::<InternalCounter>(),
            None => self
                .client
                .query(&format!(
                    r#"SELECT count() AS ?fields FROM {RSS_TABLE_NAME}
                    WHERE (
                        feed_id = toUUID(?)
                    )
                    "#,
                ))
                .bind(self.feed_id.to_string())
                .fetch_one::<InternalCounter>(),
        };

        tracing::debug!("Starting upper bound query for: {}", self.feed_id);
        let count = result_await.await?;

        tracing::debug!("Result upper bound query for: {:?}", count);
        self.upper_bound = Some(count.upper_bound as usize);

        Ok(())
    }

    pub async fn advance(&mut self) -> Result<Vec<RSSEvent>, clickhouse::error::Error> {
        // Do initial count
        self.calculate().await?;

        let offset = self.internal_offset.unwrap_or(0);

        tracing::debug!(
            "Requesting SHRSSClickStream for feed_id {} with offset {}",
            self.feed_id,
            offset
        );
        let all_events = match self.start_after {
            Some(start_after) => {
                self.client
                    .query(&format!(
                        r#"SELECT ?fields FROM {RSS_TABLE_NAME}
                           WHERE (
                               toUInt128(id) > toUInt128(toUUID(?)) AND
                               feed_id = toUUID(?)
                           )
                           ORDER BY toUInt128(id) ASC
                           OFFSET ? ROW FETCH FIRST ? ROWS ONLY"#,
                    ))
                    .bind(start_after.to_string())
                    .bind(self.feed_id.to_string())
                    .bind(offset)
                    .bind(self.per_page)
                    .fetch_all::<RSSEvent>()
                    .await?
            }
            None => {
                self.client
                    .query(&format!(
                        r#"SELECT ?fields FROM {RSS_TABLE_NAME}
                           WHERE (
                               feed_id = toUUID(?)
                           )
                           ORDER BY toUInt128(id) ASC
                           OFFSET ? ROW FETCH FIRST ? ROWS ONLY"#,
                    ))
                    .bind(self.feed_id.to_string())
                    .bind(offset)
                    .bind(self.per_page)
                    .fetch_all::<RSSEvent>()
                    .await?
            }
        };

        tracing::debug!(
            "Got {} SHRSSClickStream for feed_id {} with offset {}",
            all_events.len(),
            self.feed_id,
            offset
        );

        self.initialize = true;
        self.internal_offset = Some(offset + self.per_page);
        self.current = Some(all_events.clone());

        Ok(all_events)
    }

    // Check if exhausted
    pub fn is_exhausted(&self) -> bool {
        if self.initialize && self.current.is_none() {
            return true;
        }

        if let Some(current) = &self.current {
            current.len() < self.per_page
        } else {
            false
        }
    }

    /// Fetch all events
    pub async fn fetch_all(&mut self) -> Result<Vec<RSSEvent>, clickhouse::error::Error> {
        let mut all_events = Vec::new();
        loop {
            let events = self.advance().await?;
            if events.is_empty() || self.is_exhausted() {
                break;
            }

            all_events.extend(events);
        }

        Ok(all_events)
    }
}
