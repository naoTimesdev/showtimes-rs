use std::fmt::Debug;

use clickhouse::Client;

use crate::{m::EventKind, models::SHEvent, TABLE_NAME};

pub struct SHClickStream<
    T: serde::de::DeserializeOwned + Send + Sync + Clone + Unpin + Debug + 'static,
> {
    client: Client,
    kind: EventKind,
    // User defined
    start_after: Option<showtimes_shared::ulid::Ulid>,
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
    pub(crate) async fn init(
        client: Client,
        kind: EventKind,
    ) -> Result<Self, clickhouse::error::Error> {
        tracing::debug!("Initializing SHClickStream for kind {:?}", kind);
        let mut init = Self {
            client,
            kind,
            start_after: None,
            internal_offset: None,
            current: None,
            initialize: false,
            upper_bound: None,
            per_page: 50,
        };

        // Do initial count
        init.calculate().await?;

        Ok(init)
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

        self.start_after = Some(start_after);
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
                    r#"SELECT count() AS ?fields FROM {}
                    WHERE (
                        toUInt128(id) > toUInt128(toUUID(?)) AND
                        kind = ?
                    )
                    "#,
                    TABLE_NAME,
                ))
                .bind(start_after.to_string())
                .bind(self.kind as u8)
                .fetch_one::<InternalCounter>(),
            None => self
                .client
                .query(&format!(
                    r#"SELECT count() AS ?fields FROM {}
                    WHERE (
                        kind = ?
                    )
                    "#,
                    TABLE_NAME,
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
                        r#"SELECT ?fields FROM {}
                           WHERE (
                               toUInt128(id) > toUInt128(toUUID(?)) AND
                               kind = ?
                           )
                           ORDER BY toUInt128(id) ASC
                           OFFSET ? ROW FETCH FIRST ? ROWS ONLY"#,
                        TABLE_NAME,
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
                        r#"SELECT ?fields FROM {}
                           WHERE (
                               kind = ?
                           )
                           ORDER BY toUInt128(id) ASC
                           OFFSET ? ROW FETCH FIRST ? ROWS ONLY"#,
                        TABLE_NAME,
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

        if let Some(current) = &self.current {
            current.len() < self.per_page
        } else {
            false
        }
    }

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

// impl<T> futures::TryStream for SHClickStream<T>
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
