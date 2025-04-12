use showtimes_db::{ClientShared, DatabaseShared};

use crate::common::env_or_exit;

use super::Migration;

pub struct M20240916075925ClickhouseInit {
    client: ClientShared,
    db: DatabaseShared,
}

#[async_trait::async_trait]
impl Migration for M20240916075925ClickhouseInit {
    fn init(client: &ClientShared, db: &DatabaseShared) -> Self {
        Self {
            client: client.clone(),
            db: db.clone(),
        }
    }

    fn name(&self) -> &'static str {
        "M20240916075925ClickhouseInit"
    }

    fn timestamp(&self) -> jiff::Timestamp {
        jiff::civil::datetime(2024, 9, 16, 7, 59, 25, 0)
            .to_zoned(jiff::tz::TimeZone::UTC)
            .unwrap()
            .timestamp()
    }

    fn clone_box(&self) -> Box<dyn Migration> {
        Box::new(Self {
            client: self.client.clone(),
            db: self.db.clone(),
        })
    }

    async fn up(&self) -> anyhow::Result<()> {
        let ch_url = env_or_exit("CLICKHOUSE_URL");
        let ch_user = env_or_exit("CLICKHOUSE_USER");
        let ch_pass = std::env::var("CLICKHOUSE_PASSWORD");

        tracing::info!("Initializing ClickHouse connection...");
        let ch_client = showtimes_events::SHClickHouse::new(ch_url, ch_user, ch_pass.ok()).await?;

        tracing::info!("Creating tables...");
        ch_client.create_tables().await?;

        Ok(())
    }

    async fn down(&self) -> anyhow::Result<()> {
        let ch_url = env_or_exit("CLICKHOUSE_URL");
        let ch_user = env_or_exit("CLICKHOUSE_USER");
        let ch_pass = std::env::var("CLICKHOUSE_PASSWORD");

        tracing::info!("Initializing ClickHouse connection...");
        let ch_client = showtimes_events::SHClickHouse::new(ch_url, ch_user, ch_pass.ok()).await?;

        tracing::info!("Dropping tables...");
        ch_client.drop_tables().await?;

        Ok(())
    }
}
