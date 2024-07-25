use showtimes_db::ClientMutex;

pub(crate) mod m20240725045840_init;

#[async_trait::async_trait]
pub trait Migration {
    fn init(client: &ClientMutex) -> Self
    where
        Self: Sized;
    fn name(&self) -> &'static str;
    fn timestamp(&self) -> chrono::DateTime<chrono::Utc>;
    async fn up(&self) -> anyhow::Result<()>;
    async fn down(&self) -> anyhow::Result<()>;
    fn clone_box(&self) -> Box<dyn Migration>;
}

pub fn get_migrations(client: &ClientMutex) -> Vec<Box<dyn Migration>> {
    vec![Box::new(m20240725045840_init::M20240725045840Init::init(
        client,
    ))]
}
