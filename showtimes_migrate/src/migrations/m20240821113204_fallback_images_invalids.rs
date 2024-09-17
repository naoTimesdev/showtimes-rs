use std::path::PathBuf;

use bson::doc;
use chrono::TimeZone;
use showtimes_db::{ClientShared, DatabaseShared};
use showtimes_fs::{
    local::LocalFs,
    s3::{S3Fs, S3FsCredentials, S3PathStyle},
    FsFileKind,
};

use crate::common::env_or_exit;

use super::Migration;

pub struct M20240821113204FallbackImagesInvalids {
    client: ClientShared,
    db: DatabaseShared,
}

#[async_trait::async_trait]
impl Migration for M20240821113204FallbackImagesInvalids {
    fn init(client: &ClientShared, db: &DatabaseShared) -> Self {
        Self {
            client: client.clone(),
            db: db.clone(),
        }
    }

    fn name(&self) -> &'static str {
        "M20240821113204FallbackImagesInvalids"
    }

    fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc
            .with_ymd_and_hms(2024, 8, 21, 11, 32, 4)
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
        let manifest_dir = env!("CARGO_MANIFEST_DIR");

        tracing::info!("Creating Meilisearch client instances...");
        let meilisearch = showtimes_search::create_connection(&meili_url, &meili_key).await?;
        let s_server_index = meilisearch.index(showtimes_search::models::Server::index_name());
        let s_server_pk = showtimes_search::models::Server::primary_key();
        let s_user_index = meilisearch.index(showtimes_search::models::User::index_name());
        let s_user_pk = showtimes_search::models::User::primary_key();

        tracing::info!("Setting up filesystem...");
        let s3_bucket = std::env::var("S3_BUCKET").ok();
        let s3_region = std::env::var("S3_REGION").ok();
        let s3_endpoint_url = std::env::var("S3_ENDPOINT_URL").ok();
        let s3_access_key = std::env::var("S3_ACCESS_KEY").ok();
        let s3_secret_key = std::env::var("S3_SECRET_KEY").ok();
        let s3_path_style =
            std::env::var("S3_PATH_STYLE")
                .ok()
                .and_then(|style| match style.as_str() {
                    "virtual" => Some(S3PathStyle::VirtualHost),
                    "path" => Some(S3PathStyle::Path),
                    _ => None,
                });
        let local_storage = std::env::var("LOCAL_STORAGE").ok();

        let storages: showtimes_fs::FsPool = match (
            s3_bucket,
            s3_region,
            s3_endpoint_url,
            s3_access_key,
            s3_secret_key,
            local_storage,
        ) {
            (
                Some(bucket),
                Some(region),
                Some(endpoint_url),
                Some(access_key),
                Some(secret_key),
                _,
            ) => {
                tracing::info!(
                    " Creating S3Fs with region: {}, bucket: {}, endpoint: {:?}",
                    region,
                    bucket,
                    region,
                );

                let credentials = S3FsCredentials::new(&access_key, &secret_key);
                let bucket_info = S3Fs::make_bucket(&bucket, &endpoint_url, &region, s3_path_style);
                showtimes_fs::FsPool::S3Fs(S3Fs::new(bucket_info, credentials))
            }
            (_, _, _, _, _, Some(directory)) => {
                let dir_path = std::path::PathBuf::from(directory);

                showtimes_fs::FsPool::LocalFs(LocalFs::new(dir_path))
            }
            _ => anyhow::bail!("No storage provided"),
        };

        tracing::info!("Initializing filesystem with {}...", storages.get_name());
        storages.init().await?;

        tracing::info!("Uploading fallback images to the filesystem...");
        let assets_dir = PathBuf::from(manifest_dir).join("assets");
        let default_servers = assets_dir.join("default-servers.png");
        let default_projects = assets_dir.join("default-projects.png");
        let default_users = assets_dir.join("default-users.png");

        let file_default_servers = tokio::fs::File::open(default_servers).await?;
        let file_default_projects = tokio::fs::File::open(default_projects).await?;
        let file_default_users = tokio::fs::File::open(default_users).await?;

        tracing::info!("Uploading default servers image...");
        storages
            .file_stream_upload(
                "server",
                "default.png",
                file_default_servers,
                None,
                Some(FsFileKind::Invalids),
            )
            .await?;
        tracing::info!("Uploading default projects image...");
        storages
            .file_stream_upload(
                "project",
                "default.png",
                file_default_projects,
                None,
                Some(FsFileKind::Invalids),
            )
            .await?;
        tracing::info!("Uploading default users image...");
        storages
            .file_stream_upload(
                "user",
                "default.png",
                file_default_users,
                None,
                Some(FsFileKind::Invalids),
            )
            .await?;

        tracing::info!("Updating projects with unknown covers...");
        let users_db = showtimes_db::UserHandler::new(&self.db);
        let users = users_db
            .find_all_by(doc! {
                "avatar": null
            })
            .await?;

        tracing::info!("Updating {} users with unknown avatars...", users.len());
        users_db
            .get_collection()
            .update_many(
                doc! {
                    "avatar": null
                },
                doc! {
                    "$set": {
                        "avatar": {
                            "kind": FsFileKind::Invalids.as_path_name(),
                            "key": "user",
                            "filename": "default.png",
                            "format": "png",
                            "parent": null
                        }
                    }
                },
            )
            .await?;

        let s_users: Vec<showtimes_search::models::User> = users
            .into_iter()
            .map(showtimes_search::models::User::from)
            .collect();

        tracing::info!("Updating users in search index...");
        let task = s_user_index
            .add_or_update(&s_users, Some(s_user_pk))
            .await?;
        tracing::info!(" Waiting for search index update to complete...");
        task.wait_for_completion(&*meilisearch, None, None).await?;

        tracing::info!("Updating servers with unknown covers...");
        let servers_db = showtimes_db::ServerHandler::new(&self.db);
        let servers = servers_db
            .find_all_by(doc! {
                "avatar": null
            })
            .await?;

        tracing::info!("Updating {} servers with unknown avatars...", servers.len());
        servers_db
            .get_collection()
            .update_many(
                doc! {
                    "avatar": null
                },
                doc! {
                    "$set": {
                        "avatar": {
                            "kind": FsFileKind::Invalids.as_path_name(),
                            "key": "server",
                            "filename": "default.png",
                            "format": "png",
                            "parent": null
                        }
                    }
                },
            )
            .await?;
        let s_servers: Vec<showtimes_search::models::Server> = servers
            .into_iter()
            .map(showtimes_search::models::Server::from)
            .collect();

        tracing::info!("Updating servers in search index...");
        let task = s_server_index
            .add_or_update(&s_servers, Some(s_server_pk))
            .await?;
        tracing::info!(" Waiting for servers index update to complete...");
        task.wait_for_completion(&*meilisearch, None, None).await?;

        tracing::info!("Migration completed successfully");

        Ok(())
    }

    async fn down(&self) -> anyhow::Result<()> {
        Ok(()) // No down migration
    }
}
