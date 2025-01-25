use chrono::TimeZone;
use mongodb::bson::doc;
use showtimes_db::{ClientShared, DatabaseShared};

use crate::common::env_or_exit;

use super::Migration;

pub struct M20241026154029ProjectStatusField {
    client: ClientShared,
    db: DatabaseShared,
}

#[async_trait::async_trait]
impl Migration for M20241026154029ProjectStatusField {
    fn init(client: &ClientShared, db: &DatabaseShared) -> Self {
        Self {
            client: client.clone(),
            db: db.clone(),
        }
    }

    fn name(&self) -> &'static str {
        "M20241026154029ProjectStatusField"
    }

    fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc
            .with_ymd_and_hms(2024, 10, 26, 15, 40, 29)
            .unwrap()
    }

    fn clone_box(&self) -> Box<dyn Migration> {
        Box::new(Self {
            client: self.client.clone(),
            db: self.db.clone(),
        })
    }

    async fn up(&self) -> anyhow::Result<()> {
        let meili_url = env_or_exit("MEILI_URL");
        let meili_key = env_or_exit("MEILI_KEY");

        tracing::info!("Creating Meilisearch client instances...");
        let meilisearch = showtimes_search::create_connection(&meili_url, &meili_key).await?;
        let s_project_index = meilisearch.index(showtimes_search::models::Project::index_name());
        let s_project_pk = showtimes_search::models::Project::primary_key();

        tracing::info!("Updating all projects...");
        let project_db = showtimes_db::ProjectHandler::new(&self.db);
        let project_col = project_db.get_collection();

        let cursor_res = project_col
            .update_many(
                doc! {
                    "status": null
                },
                doc! {
                    "$set": {
                        "status": "ACTIVE"
                    }
                },
            )
            .await?;

        tracing::info!("Updated {} projects", cursor_res.modified_count);

        tracing::info!("Getting all projects to save in search index...");

        let all_active = project_db
            .find_all_by(doc! {
                "status": "ACTIVE"
            })
            .await?;

        tracing::info!("Updating projects in search index...");
        let s_projects: Vec<showtimes_search::models::Project> = all_active
            .iter()
            .map(showtimes_search::models::Project::from)
            .collect();

        let task = s_project_index
            .add_or_update(&s_projects, Some(s_project_pk))
            .await?;

        tracing::info!(" Waiting for projects index update to complete...");
        task.wait_for_completion(&*meilisearch, None, None).await?;

        tracing::info!("Updating project search index metadata...");
        showtimes_search::models::Project::update_schema(&meilisearch).await?;

        Ok(())
    }

    async fn down(&self) -> anyhow::Result<()> {
        // Ignore down migration
        Ok(())
    }
}
