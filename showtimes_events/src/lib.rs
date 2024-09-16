#![doc = include_str!("../README.md")]

use std::sync::Arc;

pub mod brokers;
pub mod models;
pub use brokers::MemoryBroker;
use clickhouse::Client;
pub use models as m;

/// The shared [`SHClickHouse`] client
pub type SharedSHClickHouse = Arc<SHClickHouse>;

const DATABASE_NAME: &str = "nt_showtimes";
const TABLE_NAME: &str = "events";

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
        let ch_client = if let Some(password) = password {
            ch_client.with_password(password)
        } else {
            ch_client
        };

        let sh_client = Self::initialize(&ch_client).await?;

        Ok(sh_client)
    }

    async fn initialize(client: &Client) -> Result<Self, clickhouse::error::Error> {
        // Ping the server
        client.query("SELECT 1").execute().await?;

        // Create the database if not exists
        client
            .query(&format!("CREATE DATABASE IF NOT EXISTS {}", DATABASE_NAME))
            .execute()
            .await?;

        let client = client.clone().with_database(DATABASE_NAME);

        Ok(Self { client })
    }

    /// Create the necessary tables in the database
    pub async fn create_tables(&self) -> Result<(), clickhouse::error::Error> {
        self.client
            .query(&format!(
                r#"
                CREATE TABLE IF NOT EXISTS {} (
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
            "#,
                TABLE_NAME
            ))
            .execute()
            .await
    }

    /// Drop all tables in the database
    pub async fn drop_tables(&self) -> Result<(), clickhouse::error::Error> {
        self.client
            .query(&format!("DROP TABLE IF EXISTS {}", TABLE_NAME))
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
        T: serde::Serialize + Send + Sync + Clone + 'static,
    {
        let data_event = m::SHEvent::new(kind, data.clone());
        let data_event = if let Some(actor) = actor {
            data_event.with_actor(actor)
        } else {
            data_event
        };

        let mut insert = self.client.insert(TABLE_NAME)?;
        insert.write(&data_event).await?;
        insert.end().await?;

        MemoryBroker::publish(data);

        Ok(())
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
        T: serde::Serialize + Send + Sync + Clone + 'static,
    {
        let client = self.client.clone();
        tokio::task::spawn(async move {
            let data_event = m::SHEvent::new(kind, data.clone());
            let data_event = if let Some(actor) = actor {
                data_event.with_actor(actor)
            } else {
                data_event
            };
            let mut insert = client.insert(TABLE_NAME)?;
            insert.write(&data_event).await?;
            insert.end().await?;

            MemoryBroker::publish(data);

            Ok(())
        })
    }
}
