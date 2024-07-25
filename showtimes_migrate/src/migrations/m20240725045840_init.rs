use chrono::TimeZone;
use showtimes_db::ClientMutex;

use super::Migration;

#[derive(Clone)]
pub struct M20240725045840Init {
    client: ClientMutex,
}

#[async_trait::async_trait]
impl Migration for M20240725045840Init {
    fn init(client: &ClientMutex) -> Self {
        Self {
            client: client.clone(),
        }
    }

    fn name(&self) -> &'static str {
        "M20240725045840Init"
    }

    fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc
            .with_ymd_and_hms(2024, 7, 25, 4, 58, 40)
            .unwrap()
    }

    async fn up(&self) -> anyhow::Result<()> {
        // TODO: Implement the up migration
        Ok(())
    }

    async fn down(&self) -> anyhow::Result<()> {
        // TODO: Implement the down migration
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Migration> {
        Box::new(self.clone())
    }
}
