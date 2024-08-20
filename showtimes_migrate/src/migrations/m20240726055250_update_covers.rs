use std::collections::HashMap;

use bson::doc;
use chrono::TimeZone;
use showtimes_db::{ClientShared, DatabaseShared};
use showtimes_fs::{
    local::LocalFs,
    s3::{S3Fs, S3FsCredentialsProvider, S3FsRegionProvider},
    FsFileKind,
};

use crate::common::env_or_exit;

use super::Migration;

pub struct M20240726055250UpdateCovers {
    client: ClientShared,
    db: DatabaseShared,
}

#[async_trait::async_trait]
impl Migration for M20240726055250UpdateCovers {
    fn init(client: &ClientShared, db: &DatabaseShared) -> Self {
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
        let meili_url = env_or_exit("MEILI_URL");
        let meili_key = env_or_exit("MEILI_KEY");

        tracing::info!("Creating Meilisearch client instances...");
        let meilisearch = showtimes_search::create_connection(&meili_url, &meili_key).await?;
        let s_project_index = meilisearch
            .lock()
            .await
            .index(showtimes_search::models::Project::index_name());
        let s_project_pk = showtimes_search::models::Project::primary_key();

        tracing::info!("Setting up filesystem...");
        let s3_bucket = std::env::var("S3_BUCKET").ok();
        let s3_region = std::env::var("S3_REGION").ok();
        let s3_endpoint_url = std::env::var("S3_ENDPOINT_URL").ok();
        let s3_access_key = std::env::var("S3_ACCESS_KEY").ok();
        let s3_secret_key = std::env::var("S3_SECRET_KEY").ok();
        let local_storage = std::env::var("LOCAL_STORAGE").ok();

        let region_info = match (s3_region, s3_endpoint_url) {
            (Some(region), Some(endpoint)) => {
                Some(S3FsRegionProvider::new(&region, Some(&endpoint)))
            }
            (Some(region), None) => Some(S3FsRegionProvider::new(&region, None)),
            _ => None,
        };

        let storages: showtimes_fs::FsPool = match (
            s3_bucket,
            region_info,
            s3_access_key,
            s3_secret_key,
            local_storage,
        ) {
            (Some(bucket), Some(region), Some(access_key), Some(secret_key), _) => {
                tracing::info!(
                    " Creating S3Fs with region: {}, bucket: {}, endpoint: {:?}",
                    region.region(),
                    bucket,
                    region.endpoint_url(),
                );

                let credentials = S3FsCredentialsProvider::new(&access_key, &secret_key);
                showtimes_fs::FsPool::S3Fs(S3Fs::new(&bucket, credentials, region).await)
            }
            (_, _, _, _, Some(directory)) => {
                let dir_path = std::path::PathBuf::from(directory);

                showtimes_fs::FsPool::LocalFs(LocalFs::new(dir_path))
            }
            _ => anyhow::bail!("No storage provided"),
        };

        tracing::info!("Initializing filesystem with {}...", storages.get_name());
        storages.init().await?;

        let project_db = showtimes_db::ProjectHandler::new(&self.db);
        let projects = project_db
            .find_all_by(doc! {
                "poster.image.key": "stubbed"
            })
            .await?;

        // Cover URL -> (Server ID, Project ID)s
        let mut locked_anilist_maps: HashMap<String, Vec<showtimes_db::m::Project>> =
            HashMap::new();
        for project in &projects {
            let cover_url = proper_url(&project.poster.image.filename);
            locked_anilist_maps
                .entry(cover_url)
                .or_default()
                .push(project.clone());
        }

        let ua_ver = format!(
            "showtimes-rs-migration/{} (+https://github.com/naoTimesdev/showtimes-rs)",
            env!("CARGO_PKG_VERSION")
        );
        let mut header_maps = reqwest::header::HeaderMap::new();
        header_maps.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_str(&ua_ver)?,
        );

        let client = reqwest::ClientBuilder::new()
            .http2_adaptive_window(true)
            .default_headers(header_maps)
            .build()?;

        // Update bit-by-bit so we don't lose progress if this crash
        for (cover_url, mut projects) in locked_anilist_maps {
            tracing::info!(" Downloading cover for: {}", cover_url);
            let cover_bytes = download_cover(&client, &cover_url).await?;

            let cover_format = cover_url
                .split('.')
                .last()
                .ok_or_else(|| anyhow::anyhow!("No cover format found"))?;

            let cover_key = format!("cover.{}", cover_format);
            for project in &mut projects {
                let image_meta = showtimes_db::m::ImageMetadata::new(
                    FsFileKind::Images.as_path_name(),
                    project.id.to_string(),
                    cover_key.to_string(),
                    cover_format.to_string(),
                    Some(project.creator.to_string()),
                );

                // Convert u8 to AsyncReadExt compatible
                tracing::info!(
                    " Uploading cover to filesystem for {} ({})",
                    project.id,
                    project.creator
                );
                let mut stream = std::io::Cursor::new(cover_bytes.clone());

                storages
                    .file_stream_upload(
                        &image_meta.key,
                        &image_meta.filename,
                        &mut stream,
                        image_meta.parent.as_deref(),
                        Some(FsFileKind::Images),
                    )
                    .await?;

                project.poster.image = image_meta;

                tracing::info!("  Updating project: {}", project.id);
                project_db.save(project, None).await?;
                tracing::info!("  Updating project in search index: {}", project.id);
                let s_project = vec![showtimes_search::models::Project::from(project.clone())];
                s_project_index
                    .add_or_update(&s_project, Some(s_project_pk))
                    .await?;
            }
        }

        Ok(())
    }

    async fn down(&self) -> anyhow::Result<()> {
        Ok(()) // No down migration
    }

    fn clone_box(&self) -> Box<dyn Migration> {
        Box::new(Self {
            client: self.client.clone(),
            db: self.db.clone(),
        })
    }
}

fn proper_url(url: &str) -> String {
    url.replace("/small/", "/large/")
        .replace("/medium/", "/large/")
        .to_string()
}

async fn download_cover(client: &reqwest::Client, url: &str) -> anyhow::Result<Vec<u8>> {
    // Check if cover exist
    let try_resolution = vec!["/large/", "/medium/", "/small/"];

    for resolution in try_resolution {
        let cover_url = url.replace("/large/", resolution);
        let response = client.get(&cover_url).send().await?;

        if response.status().is_success() {
            tracing::info!("  Downloaded cover from: {}", cover_url);
            let content_data = response.bytes().await?;
            let bytes_map = content_data.to_vec();

            return Ok(bytes_map);
        }
    }

    anyhow::bail!("Cover not found")
}
