#![doc = include_str!("../README.md")]

pub mod brokers;
pub mod models;
pub use brokers::MemoryBroker;
use clickhouse::Client;
pub use models as m;

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
            .query("CREATE DATABASE IF NOT EXISTS showtimes")
            .execute()
            .await?;

        let client = client.clone().with_database("showtimes");
        client
            .query(
                r#"
                CREATE TABLE IF NOT EXISTS events (
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
                    )
                    data STRING,
                    timestamp DateTime
                ) ENGINE = MergeTree()
                ORDER BY (id)
            "#,
            )
            .execute()
            .await?;

        Ok(Self { client })
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
    ) -> Result<(), clickhouse::error::Error>
    where
        T: serde::Serialize + Send + Sync + Clone + 'static,
    {
        let data_event = m::SHEvent::new(kind, data.clone());

        let mut insert = self.client.insert("events")?;
        insert.write(&data_event).await?;
        insert.end().await?;

        MemoryBroker::publish(data);

        Ok(())
    }
}
