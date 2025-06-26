#![warn(missing_docs, clippy::empty_docs, rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../README.md")]

use std::fmt::Debug;
use std::sync::Arc;

pub mod brokers;
pub mod models;
mod streams;
pub use brokers::MemoryBroker;
pub use brokers::RSSBroker;
use clickhouse::Client;
pub use clickhouse::error::Error as ClickHouseError;
pub use models as m;

/// The shared [`SHClickHouse`] client
pub type SharedSHClickHouse = Arc<SHClickHouse>;

const DATABASE_NAME: &str = "nt_showtimes";
pub(crate) const TABLE_NAME: &str = "events";
pub(crate) const RSS_TABLE_NAME: &str = "rss_feed";

/// The main ClickHouse client handler for Showtimes
pub struct SHClickHouse {
    client: Client,
}

impl SHClickHouse {
    /// Create a new instance of [`SHClickHouse`]
    pub async fn new(
        url: impl Into<String>,
        username: impl Into<String>,
        password: Option<impl Into<String>>,
    ) -> Result<Self, clickhouse::error::Error> {
        let ch_client = Client::default().with_url(url).with_user(username);
        let ch_client = match password {
            Some(password) => ch_client.with_password(password),
            _ => ch_client,
        };

        let sh_client = Self::initialize(&ch_client).await?;

        Ok(sh_client)
    }

    async fn initialize(client: &Client) -> Result<Self, clickhouse::error::Error> {
        // Ping the server
        client.query("SELECT 1").execute().await?;

        // Create the database if not exists
        client
            .query(&format!("CREATE DATABASE IF NOT EXISTS {DATABASE_NAME}"))
            .execute()
            .await?;

        let client = client.clone().with_database(DATABASE_NAME);

        Ok(Self { client })
    }

    /// Create the necessary tables in the database
    pub async fn create_tables(&self) -> Result<(), clickhouse::error::Error> {
        // Events table
        self.client
            .query(&format!(
                r#"
                CREATE TABLE IF NOT EXISTS {TABLE_NAME} (
                    id UUID,
                    kind Enum8(
                        'user_created' = 1,
                        'user_updated' = 2,
                        'user_deleted' = 3,
                        'server_created' = 10,
                        'server_updated' = 11,
                        'server_deleted' = 12,
                        'project_created' = 20,
                        'project_updated' = 21,
                        'project_deleted' = 22,
                        'project_episodes' = 30,
                        'collaboration_created' = 40,
                        'collaboration_accepted' = 41,
                        'collaboration_rejected' = 42,
                        'collaboration_deleted' = 43,
                        'collaboration_retracted' = 44,
                    ),
                    data String,
                    actor Nullable(String),
                    timestamp DateTime
                ) ENGINE = MergeTree()
                ORDER BY (timestamp)
            "#
            ))
            .execute()
            .await?;

        // RSS table
        self.client
            .query(&format!(
                r#"
                CREATE TABLE IF NOT EXISTS {RSS_TABLE_NAME} (
                    id UUID,
                    feed_id UUID,
                    server_id UUID,
                    hash String,
                    entries String,
                    timestamp DateTime
                ) ENGINE = MergeTree()
                ORDER BY (timestamp)
                "#,
            ))
            .execute()
            .await
    }

    /// Drop all tables in the database
    pub async fn drop_tables(&self) -> Result<(), clickhouse::error::Error> {
        self.client
            .query(&format!("DROP TABLE IF EXISTS {TABLE_NAME}"))
            .execute()
            .await?;

        self.client
            .query(&format!("DROP TABLE IF EXISTS {RSS_TABLE_NAME}"))
            .execute()
            .await
    }

    /// Create new event, this will also forward the event to the broker
    /// for other services to consume
    ///
    /// # Arguments
    /// * `kind` - The kind of event
    /// * `data` - The data of the event
    pub async fn create_event<T>(
        &self,
        kind: m::EventKind,
        data: T,
        actor: Option<String>,
    ) -> Result<(), clickhouse::error::Error>
    where
        T: serde::Serialize + Send + Sync + Clone + Debug + 'static,
    {
        self.create_event_many(kind, vec![data], actor).await
    }

    /// A similar function to [`SHClickHouse::create_event`] but will run
    /// on non-blocking manner or in another thread
    pub fn create_event_async<T>(
        &self,
        kind: m::EventKind,
        data: T,
        actor: Option<String>,
    ) -> tokio::task::JoinHandle<Result<(), clickhouse::error::Error>>
    where
        T: serde::Serialize + Send + Sync + Clone + Debug + 'static,
    {
        self.create_event_many_async(kind, vec![data], actor)
    }

    /// Create a new event with multiple data, this will also forward the event to the broker
    /// for other services to consume, similar to [`SHClickHouse::create_event`] but with multiple data
    ///
    /// # Arguments
    /// * `kind` - The kind of event
    /// * `data` - The data of the event
    pub async fn create_event_many<T>(
        &self,
        kind: m::EventKind,
        data: Vec<T>,
        actor: Option<String>,
    ) -> Result<(), clickhouse::error::Error>
    where
        T: serde::Serialize + Send + Sync + Clone + Debug + 'static,
    {
        let all_events: Vec<models::SHEvent<T>> = data
            .iter()
            .map(|d| make_event(kind, d, actor.clone()))
            .collect::<Vec<_>>();

        push_event(&self.client, kind, &all_events).await?;

        for ev in all_events {
            // Publish one by one
            MemoryBroker::publish(ev);
        }

        Ok(())
    }

    /// A similar function to [`SHClickHouse::create_event_many`] but will run
    /// on non-blocking manner or in another thread
    pub fn create_event_many_async<T>(
        &self,
        kind: m::EventKind,
        data: Vec<T>,
        actor: Option<String>,
    ) -> tokio::task::JoinHandle<Result<(), clickhouse::error::Error>>
    where
        T: serde::Serialize + Send + Sync + Clone + Debug + 'static,
    {
        let client = self.client.clone();
        tokio::task::spawn(async move {
            let all_events: Vec<models::SHEvent<T>> = data
                .iter()
                .map(|d| make_event(kind, d, actor.clone()))
                .collect::<Vec<_>>();

            push_event(&client, kind, &all_events).await?;

            for ev in all_events {
                // Publish one by one
                MemoryBroker::publish(ev);
            }

            Ok(())
        })
    }

    /// Create a new RSS event with multiple data, this will also forward the event to the broker
    /// for other services to consume, similar to [`SHClickHouse::create_event`] but with multiple data
    ///
    /// # Arguments
    /// * `data` - The data of the RSS event
    pub async fn create_rss_many<T>(
        &self,
        data: Vec<crate::m::RSSEvent>,
    ) -> Result<(), clickhouse::error::Error> {
        push_rss(&self.client, &data).await?;

        for ev in data {
            // Publish one by one
            RSSBroker::publish(ev.feed_id(), ev);
        }

        Ok(())
    }

    /// A similar function to [`SHClickHouse::create_rss_many`] but will run
    /// on non-blocking manner or in another thread
    pub fn create_rss_many_async(
        &self,
        data: Vec<crate::m::RSSEvent>,
    ) -> tokio::task::JoinHandle<Result<(), clickhouse::error::Error>> {
        let client = self.client.clone();
        tokio::task::spawn(async move {
            push_rss(&client, &data).await?;

            for ev in data {
                // Publish one by one
                RSSBroker::publish(ev.feed_id(), ev);
            }

            Ok(())
        })
    }

    /// Query the events from the database with proper pagination
    pub fn query<T>(&self, kind: m::EventKind) -> streams::SHClickStream<T>
    where
        T: serde::de::DeserializeOwned + Send + Sync + Clone + Unpin + std::fmt::Debug + 'static,
    {
        streams::SHClickStream::init(self.client.clone(), kind)
    }

    /// Query the RSS events from the database with proper pagination
    pub fn query_rss(&self, feed_id: showtimes_shared::ulid::Ulid) -> streams::SHRSSClickStream {
        streams::SHRSSClickStream::init(self.client.clone(), feed_id)
    }

    /// Get single or latest RSS event for a feed from the database
    pub async fn get_latest_rss(
        &self,
        feed_id: showtimes_shared::ulid::Ulid,
    ) -> Result<Option<models::RSSEvent>, clickhouse::error::Error> {
        let results = self
            .client
            .query(&format!(
                r#"SELECT ?fields FROM {RSS_TABLE_NAME}
                   WHERE (
                       feed_id = toUUID(?)
                   )
                   ORDER BY toUInt128(id) DESC
                   LIMIT 1"#,
            ))
            .bind(feed_id.to_string())
            .fetch_all::<models::RSSEvent>()
            .await?;

        if let Some(result) = results.first() {
            Ok(Some(result.clone()))
        } else {
            Ok(None)
        }
    }
}

/// Wrap the event data into [`m::SHEvent`] and return it
fn make_event<T>(kind: m::EventKind, data: &T, actor: Option<String>) -> m::SHEvent<T>
where
    T: serde::Serialize + Send + Sync + Clone + 'static,
{
    let data_event = m::SHEvent::new(kind, data.clone());
    if let Some(actor) = actor {
        data_event.with_actor(actor)
    } else {
        data_event
    }
}

/// The actual event pusher to ClickHouse
///
/// Support for multiple data
///
/// # Arguments
/// * `client` - The ClickHouse client
/// * `kind` - The kind of event
/// * `data` - The data of the event
/// * `actor` - The actor of the event
async fn push_event<T>(
    client: &Client,
    kind: m::EventKind,
    data: &[crate::m::SHEvent<T>],
) -> Result<(), clickhouse::error::Error>
where
    T: serde::Serialize + Send + Sync + Clone + Debug + 'static,
{
    tracing::debug!(
        "Preparing to push event \"{:?}\" to ClickHouse (table = {}, db = {})",
        kind,
        TABLE_NAME,
        DATABASE_NAME
    );
    let mut insert = client.insert(TABLE_NAME)?;
    for d in data {
        // let event = make_event(kind, d, actor.clone());
        insert.write(d).await?;
    }
    tracing::debug!(
        "Inserting event \"{:?}\" to ClickHouse with {} event(s) (table = {}, db = {})",
        kind,
        data.len(),
        TABLE_NAME,
        DATABASE_NAME,
    );
    insert.end().await?;

    Ok(())
}

async fn push_rss(
    client: &Client,
    data: &[crate::m::RSSEvent],
) -> Result<(), clickhouse::error::Error> {
    tracing::debug!(
        "Preparing to push RSS event to ClickHouse (table = {}, db = {})",
        RSS_TABLE_NAME,
        DATABASE_NAME
    );
    let mut insert = client.insert(RSS_TABLE_NAME)?;
    for d in data {
        insert.write(d).await?;
    }
    tracing::debug!(
        "Inserting RSS event with {} event(s) (table = {}, db = {})",
        data.len(),
        RSS_TABLE_NAME,
        DATABASE_NAME
    );
    insert.end().await?;

    Ok(())
}
