use showtimes_db::{ClientShared, DatabaseShared};

pub(crate) mod m20240725045840_init;
pub(crate) mod m20240726055250_update_covers;
pub(crate) mod m20240821113204_fallback_images_invalids;

#[async_trait::async_trait]
pub trait Migration {
    fn init(client: &ClientShared, db: &DatabaseShared) -> Self
    where
        Self: Sized;
    fn name(&self) -> &'static str;
    fn timestamp(&self) -> chrono::DateTime<chrono::Utc>;
    async fn up(&self) -> anyhow::Result<()>;
    async fn down(&self) -> anyhow::Result<()>;
    fn clone_box(&self) -> Box<dyn Migration>;
}

pub fn get_migrations(client: &ClientShared, db: &DatabaseShared) -> Vec<Box<dyn Migration>> {
    vec![
        Box::new(m20240725045840_init::M20240725045840Init::init(client, db)),
        Box::new(m20240726055250_update_covers::M20240726055250UpdateCovers::init(client, db)),
        Box::new(
            m20240821113204_fallback_images_invalids::M20240821113204FallbackImagesInvalids::init(
                client, db,
            ),
        ),
    ]
}
