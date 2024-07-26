use chrono::TimeZone;
use showtimes_db::{ClientMutex, DatabaseMutex};

use super::Migration;

pub struct M20240726055250UpdateCovers {
    client: ClientMutex,
    db: DatabaseMutex,
}

#[async_trait::async_trait]
impl Migration for M20240726055250UpdateCovers {
    fn init(client: &ClientMutex, db: &DatabaseMutex) -> Self {
        Self {
            client: client.clone(),
            db: db.clone(),
        }
    }

    fn name(&self) -> &'static str {
        "M20240726055250UpdateCovers"
    }

    fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc
            .with_ymd_and_hms(2024, 7, 26, 5, 52, 50)
            .unwrap()
    }

    async fn up(&self) -> anyhow::Result<()> {
        // TODO: Implement the up migration
        anyhow::bail!("Not implemented")
    }

    async fn down(&self) -> anyhow::Result<()> {
        // TODO: Implement the down migration
        anyhow::bail!("Not implemented")
    }

    fn clone_box(&self) -> Box<dyn Migration> {
        Box::new(Self {
            client: self.client.clone(),
            db: self.db.clone(),
        })
    }
}
