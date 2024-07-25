use std::{collections::HashMap, str::FromStr};

use bson::doc;
use chrono::TimeZone;
use futures::TryStreamExt;
use showtimes_db::{
    m::{DiscordUser, User},
    ClientMutex, DatabaseMutex, UserHandler,
};
use showtimes_fs::{local::LocalFs, s3::S3Fs};

use crate::{common::env_or_exit, models::projects::ProjectAssignee};

use super::Migration;

fn is_discord_snowflake(value: &str) -> bool {
    match value.parse::<u64>() {
        Ok(data) => data >= 4194304,
        Err(_) => false,
    }
}

#[derive(Clone)]
pub struct M20240725045840Init {
    client: ClientMutex,
    db: DatabaseMutex,
}

#[async_trait::async_trait]
impl Migration for M20240725045840Init {
    fn init(client: &ClientMutex, db: &DatabaseMutex) -> Self {
        Self {
            client: client.clone(),
            db: db.clone(),
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
        let meili_url = env_or_exit("MEILI_URL");
        let meili_key = env_or_exit("MEILI_KEY");
        let old_db_name = env_or_exit("OLD_DB_NAME");

        let meilisearch = M20240725045840Init::setup_meilisearch(&meili_url, &meili_key).await?;

        tracing::info!("Setting up filesystem...");
        let s3_bucket = std::env::var("S3_BUCKET").ok();
        let s3_region = std::env::var("S3_REGION").ok();
        let s3_access_key = std::env::var("S3_ACCESS_KEY").ok();
        let s3_secret_key = std::env::var("S3_SECRET_KEY").ok();
        let local_storage = std::env::var("LOCAL_STORAGE").ok();

        let storages: showtimes_fs::FsPool = match (
            s3_bucket,
            s3_region,
            s3_access_key,
            s3_secret_key,
            local_storage,
        ) {
            (Some(bucket), Some(region), Some(access_key), Some(secret_key), _) => {
                let region = showtimes_fs::Region::from_str(&region)?;
                showtimes_fs::FsPool::S3Fs(S3Fs::new(&bucket, &access_key, &secret_key, region))
            }
            (_, _, _, _, Some(directory)) => {
                let dir_path = std::path::PathBuf::from(directory);

                showtimes_fs::FsPool::LocalFs(LocalFs::new(dir_path))
            }
            _ => anyhow::bail!("No storage provided"),
        };

        tracing::info!("Initializing filesystem with {}...", storages.get_name());
        storages.init().await?;

        tracing::info!("Collecting servers...");
        let servers = self.collect_servers(&old_db_name).await?;
        tracing::info!("Found {} servers", servers.len());

        tracing::info!("Collecting valid users...");
        let users = self.collect_valid_users(&old_db_name, &servers).await?;

        // Commit users
        tracing::info!("Found {} valid users, committing to database", users.len());
        let mut users = users
            .iter()
            .map(|(_, user)| user.clone())
            .collect::<Vec<_>>();
        let user_handler = UserHandler::new(self.db.clone()).await;
        user_handler.insert(&mut users).await?;
        tracing::info!("Committed all users data, creating search data for users");

        let mapped_users: Vec<showtimes_search::models::User> =
            users.iter().map(|user| user.clone().into()).collect();

        let m_user_index = meilisearch
            .lock()
            .await
            .index(showtimes_search::models::User::index_name());
        let m_use_commit = m_user_index
            .add_documents(
                &mapped_users,
                Some(showtimes_search::models::User::primary_key()),
            )
            .await?;
        tracing::info!("Committed all users search data, waiting for commit to finish");
        m_use_commit
            .wait_for_completion(&*meilisearch.lock().await, None, None)
            .await?;

        Ok(())
    }

    async fn down(&self) -> anyhow::Result<()> {
        let meili_url = env_or_exit("MEILI_URL");
        let meili_key = env_or_exit("MEILI_KEY");

        let meilisearch = M20240725045840Init::setup_meilisearch(&meili_url, &meili_key).await?;

        // Remove users from the database
        let user_handler = UserHandler::new(self.db.clone()).await;

        tracing::info!("Dropping users collection...");
        user_handler.delete_all().await?;

        // Remove users from the search index
        tracing::info!("Dropping users search index...");
        let m_user_index = meilisearch
            .lock()
            .await
            .index(showtimes_search::models::User::index_name());

        let m_user_cm = m_user_index.delete_all_documents().await?;
        tracing::info!(" Waiting for users search index to be completely deleted...");
        m_user_cm
            .wait_for_completion(&*meilisearch.lock().await, None, None)
            .await?;
        tracing::info!("Users search index has been deleted");

        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Migration> {
        Box::new(self.clone())
    }
}

impl M20240725045840Init {
    async fn setup_meilisearch(
        meili_url: &str,
        meili_key: &str,
    ) -> anyhow::Result<showtimes_search::ClientMutex> {
        tracing::info!("Creating Meilisearch client instances...");
        let client = showtimes_search::create_connection(meili_url, meili_key).await?;

        tracing::info!("Creating Meilisearch indexes...");
        // This will create the index if it doesn't exist
        showtimes_search::models::Project::get_index(&client).await?;
        showtimes_search::models::Server::get_index(&client).await?;
        showtimes_search::models::User::get_index(&client).await?;

        tracing::info!("Updating Meilisearch indexes schema information...");
        showtimes_search::models::Project::update_schema(&client).await?;
        showtimes_search::models::Server::update_schema(&client).await?;
        showtimes_search::models::User::update_schema(&client).await?;

        Ok(client)
    }

    async fn collect_servers(
        &self,
        old_db_name: &str,
    ) -> anyhow::Result<Vec<crate::models::servers::Server>> {
        let client = self.client.lock().await;
        let db = client.database(old_db_name);

        let coll = db.collection::<crate::models::servers::Server>("showtimesdatas");
        let cursor = coll.find(doc! {}).await?;
        let results: Vec<crate::models::servers::Server> = cursor.try_collect().await?;

        Ok(results)
    }

    async fn collect_valid_users(
        &self,
        old_db_name: &str,
        servers: &[crate::models::servers::Server],
    ) -> anyhow::Result<HashMap<String, User>> {
        let client = self.client.lock().await;
        let db = client.database(old_db_name);

        let show_admins = db.collection::<crate::models::servers::ServerAdmin>("showtimesadmin");
        let ui_logins = db.collection::<crate::models::users::User>("showtimesuilogin");

        tracing::info!(" Processing from showtimesuilogin...");
        let mut ui_cursor = ui_logins
            .find(doc! { "discord_meta": {"$exists": true} })
            .await?;
        let mut all_users = HashMap::new();

        while let Some(user) = ui_cursor.try_next().await? {
            let discord_meta = user.discord_meta.unwrap();
            let discord_user = DiscordUser {
                id: discord_meta.id.clone(),
                username: discord_meta.name,
                access_token: discord_meta.access_token,
                refresh_token: discord_meta.refresh_token,
                expires_at: discord_meta.expires_at,
                avatar: None,
            };

            let created_user = match user.privilege.as_str() {
                "admin" | "owner" => User::new_admin(user.name.unwrap_or(user.id), discord_user),
                _ => User::new(user.name.unwrap_or(user.id), discord_user),
            };

            all_users.insert(discord_meta.id, created_user);
        }

        // Now create show_admins
        tracing::info!(" Processing from showtimesadmin...");
        let mut show_admins_cursor = show_admins.find(doc! {}).await?;
        while let Some(admin) = show_admins_cursor.try_next().await? {
            // If not exists, create a new user
            if !all_users.contains_key(&admin.id) {
                let mut discord_user = DiscordUser::stub();
                discord_user.id = admin.id.clone();
                let created_user = User::new(admin.id.clone(), discord_user);
                all_users.insert(admin.id.clone(), created_user);
            }
        }

        // Loop through all servers to find the owners
        tracing::info!(" Processing from servers...");
        for server in servers {
            for owner in &server.serverowner {
                if !all_users.contains_key(owner) && is_discord_snowflake(owner) {
                    let mut discord_user = DiscordUser::stub();
                    discord_user.id = owner.clone();
                    let created_user = User::new(owner.clone(), discord_user);
                    all_users.insert(owner.clone(), created_user);
                }
            }

            // Loop through projects in assignments
            for project in server.anime.iter() {
                let assignments = &project.assignments;
                let tl = &assignments.translator;
                let tlc = &assignments.translation_checker;
                let ed = &assignments.editor;
                let enc = &assignments.encoder;
                let tm = &assignments.timer;
                let ts = &assignments.typesetter;
                let qc = &assignments.quality_checker;

                let mut all_assignees = HashMap::new();
                if let Some((id, name)) = transform_assignee(tl) {
                    all_assignees.insert(id, name);
                }
                if let Some((id, name)) = transform_assignee(tlc) {
                    all_assignees.insert(id, name);
                }
                if let Some((id, name)) = transform_assignee(ed) {
                    all_assignees.insert(id, name);
                }
                if let Some((id, name)) = transform_assignee(enc) {
                    all_assignees.insert(id, name);
                }
                if let Some((id, name)) = transform_assignee(tm) {
                    all_assignees.insert(id, name);
                }
                if let Some((id, name)) = transform_assignee(ts) {
                    all_assignees.insert(id, name);
                }
                if let Some((id, name)) = transform_assignee(qc) {
                    all_assignees.insert(id, name);
                }

                for custom in &assignments.custom {
                    if let Some((id, name)) = transform_assignee(&custom.person) {
                        all_assignees.insert(id, name);
                    }
                }

                for (assignee, name) in all_assignees {
                    if !all_users.contains_key(&assignee) && is_discord_snowflake(&assignee) {
                        let mut discord_user = DiscordUser::stub();
                        discord_user.id = assignee.clone();
                        if !name.is_empty() {
                            discord_user.username = name.clone();
                        }
                        let created_user = User::new(assignee.clone(), discord_user);
                        let name = if name.is_empty() {
                            assignee.clone()
                        } else {
                            name
                        };
                        all_users.insert(name, created_user);
                    }
                }
            }
        }

        Ok(all_users)
    }
}

fn transform_assignee(assignee: &ProjectAssignee) -> Option<(String, String)> {
    match (&assignee.id, &assignee.name) {
        (Some(id), Some(name)) => Some((id.clone(), name.clone())),
        (Some(id), _) => Some((id.clone(), String::new())),
        _ => None,
    }
}
