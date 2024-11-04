//! A S3-compatible client for accessing filesystem.

use std::{sync::Arc, time::Duration};

use crate::{make_file_path, FsFileKind, FsFileObject};
use futures_util::TryStreamExt;
use rusty_s3::{
    actions::{
        CompleteMultipartUpload, CreateBucket, CreateMultipartUpload, DeleteObjects, HeadBucket,
        HeadObject, ListObjectsV2, ObjectIdentifier, UploadPart,
    },
    S3Action,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

pub use rusty_s3::{Bucket, Credentials as S3FsCredentials, UrlStyle as S3PathStyle};

const MAX_KEYS: usize = 500;
const CHUNK_SIZE: usize = 4_194_304; // 4 * 1024 * 1024 = 4MiB
const ONE_HOUR: Duration = Duration::from_secs(3600);

/// A S3-compatible client for accessing filesystem.
#[derive(Debug, Clone)]
pub struct S3Fs {
    /// Shared HTTP client.
    client: Arc<reqwest::Client>,
    /// Region information
    bucket: Bucket,
    /// Credentials
    credentials: S3FsCredentials,
}

impl S3Fs {
    /// Create a new instance of [`S3Fs`] filesystem implementation.
    ///
    /// # Parameters
    /// * `bucket`: The bucket information.
    /// * `credentials`: The credentials provider.
    pub fn new(bucket: Bucket, credentials: S3FsCredentials) -> Self {
        let ua = format!("showtimes-fs-rs/{}", env!("CARGO_PKG_VERSION"));
        let client = reqwest::ClientBuilder::new()
            .user_agent(ua)
            .http2_adaptive_window(true)
            .use_rustls_tls()
            .build()
            .unwrap();

        Self {
            client: Arc::new(client),
            bucket,
            credentials,
        }
    }

    /// Create a new instance of [`Bucket`]
    pub fn make_bucket(
        name: impl Into<String>,
        endpoint: impl Into<String>,
        region: impl Into<String>,
        path_style: Option<S3PathStyle>,
    ) -> Bucket {
        let path_style = path_style.unwrap_or(S3PathStyle::VirtualHost);
        let endpoint: String = endpoint.into();
        let name: String = name.into();
        let region: String = region.into();
        Bucket::new(
            reqwest::Url::parse(&endpoint).unwrap(),
            path_style,
            name,
            region,
        )
        .unwrap()
    }

    pub(crate) async fn init(&self) -> anyhow::Result<()> {
        // Check if the bucket exists
        tracing::debug!(
            "Initializing, checking if bucket exists: {}",
            self.bucket.name()
        );
        let head_action = HeadBucket::new(&self.bucket, Some(&self.credentials));
        let signed_url = head_action.sign(ONE_HOUR);

        let response = self.client.head(signed_url).send().await?;

        // Check status code
        if response.status().is_success() {
            tracing::debug!(
                "Bucket {} found, initialization complete",
                self.bucket.name()
            );
            Ok(())
        } else {
            // Ensure it's 404
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                // Create bucket
                tracing::debug!("Bucket {} not found, creating", self.bucket.name());
                let create_action = CreateBucket::new(&self.bucket, &self.credentials);
                let signed_url = create_action.sign(ONE_HOUR);
                let response = self.client.put(signed_url).send().await?;

                response.error_for_status()?;

                tracing::debug!("Bucket {} created", self.bucket.name());

                Ok(())
            } else {
                tracing::error!("Failed to check bucket: {}", response.status());
                anyhow::bail!("Failed to check bucket: {}", response.status());
            }
        }
    }

    pub(crate) async fn file_stat(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        filename: impl Into<String> + std::marker::Send,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<FsFileObject> {
        let filename: String = filename.into();
        let key = make_file_path(&base_key.into(), &filename, parent_id, kind.clone());

        tracing::debug!("Checking file stat for: {}", &key);
        let head_action = HeadObject::new(&self.bucket, Some(&self.credentials), &key);
        let signed_url = head_action.sign(ONE_HOUR);

        let response = self
            .client
            .head(signed_url)
            .send()
            .await?
            .error_for_status()?;

        let content_length: i64 = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .unwrap_or(-1);

        // Last modified
        let last_modified = response
            .headers()
            .get(reqwest::header::LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| {
                // Parse with chrono
                chrono::DateTime::parse_from_rfc2822(v).ok()
            })
            .map(|v| v.with_timezone(&chrono::Utc));

        // Content type
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_string())
            .unwrap_or_else(|| {
                // Guess from filename
                mime_guess::from_path(&filename)
                    .first_or_octet_stream()
                    .to_string()
            });

        let fs_meta = FsFileObject {
            filename: key.clone(),
            content_type,
            size: content_length,
            last_modified,
        };

        tracing::debug!("File stat for {}: {:?}", &key, fs_meta);

        Ok(fs_meta)
    }

    pub(crate) async fn file_exists(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        filename: impl Into<String> + std::marker::Send,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<bool> {
        let base_key: String = base_key.into();
        let filename: String = filename.into();
        let key = make_file_path(&base_key, &filename, parent_id, kind.clone());
        tracing::debug!("Checking file existence for: {}", &key);
        let result = self.file_stat(base_key, filename, parent_id, kind).await?;

        Ok(result.size > 0)
    }

    pub(crate) async fn file_stream_upload<R>(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        filename: impl Into<String> + std::marker::Send,
        stream: &mut R,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<FsFileObject>
    where
        R: AsyncReadExt + AsyncSeekExt + Send + Unpin + 'static,
    {
        let filename: String = filename.into();
        let base_key = base_key.into();
        let key = make_file_path(&base_key, &filename, parent_id, kind.clone());
        let guessed = mime_guess::from_path(&filename);
        let content_type = guessed.first_or_octet_stream().to_string();

        self.file_stream_upload_internal(&key, &content_type, stream)
            .await?;

        tracing::debug!("File uploaded: {}", &key);
        let file_stat = self
            .file_stat(&base_key, &filename, parent_id, kind)
            .await?;

        Ok(file_stat)
    }

    async fn file_stream_upload_internal<R>(
        &self,
        key: &str,
        content_type: &str,
        stream: &mut R,
    ) -> anyhow::Result<()>
    where
        R: AsyncReadExt + AsyncSeekExt + Send + Unpin + 'static,
    {
        // Seek to the end of the file to get its size in bytes
        stream.seek(std::io::SeekFrom::End(0)).await?;
        let max_position = stream.stream_position().await?;
        // Seek to the start of the file again
        stream.seek(std::io::SeekFrom::Start(0)).await?;

        tracing::debug!("Preparing to upload file: {}", key);
        // Create a multipart upload request
        let action_base = CreateMultipartUpload::new(&self.bucket, Some(&self.credentials), key);
        let start_url = action_base.sign(ONE_HOUR);

        // Send the request and get the response
        let start_resp = self
            .client
            .post(start_url)
            .header("content-type", content_type)
            .send()
            .await?
            .error_for_status()?;
        let start_body = start_resp.text().await?;

        // Parse the response to get the upload ID
        let multipart = CreateMultipartUpload::parse_response(&start_body)?;

        tracing::debug!("Got multipart upload id: {}", multipart.upload_id());

        // Initialize the part number and the array to store the ETags
        let mut part_number = 1;
        let mut etag_data: Vec<String> = vec![];

        // Upload the file in chunks of 4MB each
        while stream.stream_position().await? <= max_position {
            // Read a chunk of the file
            let mut chunks_buffer = Vec::with_capacity(CHUNK_SIZE); // 4MB
            match stream.read_exact(&mut chunks_buffer).await {
                Ok(_) => (),
                Err(e) => {
                    // Depending on the error, we silently ignore it.
                    if e.kind() != std::io::ErrorKind::UnexpectedEof {
                        return Err(e.into());
                    }
                }
            }

            let position = stream.stream_position().await?;

            if chunks_buffer.is_empty() && position >= max_position {
                // We reached the end of the file and no data was read
                break;
            }

            // Upload the chunk
            let part_upload = UploadPart::new(
                &self.bucket,
                Some(&self.credentials),
                key,
                part_number,
                multipart.upload_id(),
            );
            let part_url = part_upload.sign(ONE_HOUR);

            tracing::debug!(
                "Uploading part {} for \"{}\" with {} bytes",
                part_number,
                key,
                chunks_buffer.len()
            );
            let part_response = self
                .client
                .put(part_url)
                .body(chunks_buffer)
                .send()
                .await?
                .error_for_status()?;

            // Get the ETag from the response
            let temp_etag = part_response
                .headers()
                .get(reqwest::header::ETAG)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Failed to get etag from part upload response number {}",
                        part_number
                    )
                })?;

            // Store the ETag
            etag_data.push(temp_etag.to_str()?.to_string());
            part_number += 1;
        }

        // If there are any parts, complete the multipart upload
        if !etag_data.is_empty() {
            tracing::debug!(
                "Completing multipart upload for \"{}\" with total {} parts",
                key,
                etag_data.len()
            );
            // Create the complete request
            let deref_etag_data = etag_data.iter().map(|v| v.as_str()).collect::<Vec<&str>>();
            let complete_action = CompleteMultipartUpload::new(
                &self.bucket,
                Some(&self.credentials),
                key,
                multipart.upload_id(),
                deref_etag_data.into_iter(),
            );

            let complete_url = complete_action.sign(ONE_HOUR);

            // Send the complete request
            self.client
                .post(complete_url)
                .body(complete_action.body())
                .send()
                .await?
                .error_for_status()?;
        }

        Ok(())
    }

    pub(crate) async fn file_stream_download<'wlife, W: AsyncWriteExt + Unpin + Send>(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        filename: impl Into<String> + std::marker::Send,
        writer: &'wlife mut W,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<()> {
        let filename: String = filename.into();
        let key = make_file_path(&base_key.into(), &filename, parent_id, kind.clone());

        let action = rusty_s3::actions::GetObject::new(&self.bucket, Some(&self.credentials), &key);
        let signed_url = action.sign(ONE_HOUR);

        tracing::debug!("Downloading file stream for: {}", &key);
        let response = self.client.get(signed_url).send().await?;

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.try_next().await? {
            writer.write_all(&chunk).await?;
        }

        Ok(())
    }

    pub(crate) async fn file_delete(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        filename: impl Into<String> + std::marker::Send,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<()> {
        let filename: String = filename.into();
        let key = make_file_path(&base_key.into(), &filename, parent_id, kind.clone());

        let action =
            rusty_s3::actions::DeleteObject::new(&self.bucket, Some(&self.credentials), &key);
        let signed_url = action.sign(ONE_HOUR);

        tracing::debug!("Deleting file: {}", &key);
        self.client
            .delete(signed_url)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub(crate) async fn directory_delete(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<()> {
        let mut last_key: Option<String> = None;
        let mut stop = false;
        let prefix = make_file_path(&base_key.into(), "", parent_id, kind);
        tracing::debug!("Preparing to delete directory: {}", &prefix);

        while !stop {
            let mut action =
                rusty_s3::actions::ListObjectsV2::new(&self.bucket, Some(&self.credentials));
            action.with_max_keys(MAX_KEYS);
            action.with_prefix(&prefix);
            if let Some(last_key) = &last_key {
                action.with_start_after(last_key);
            }

            let signed_url = action.sign(ONE_HOUR);

            tracing::debug!(
                "Listing objects for deletion: {} (last key? = {:?})",
                &prefix,
                &last_key
            );
            let response = self
                .client
                .get(signed_url)
                .send()
                .await?
                .error_for_status()?;
            let text_data = response.text().await?;

            let parsed = ListObjectsV2::parse_response(&text_data)?;

            let delete_keys: Vec<ObjectIdentifier> = parsed
                .contents
                .iter()
                .map(|obj| ObjectIdentifier::new(obj.key.clone()))
                .collect();

            if delete_keys.is_empty() {
                tracing::debug!(
                    "No more objects to delete for: {} (last key? = {:?})",
                    &prefix,
                    &last_key
                );
                break;
            }

            let del_action =
                DeleteObjects::new(&self.bucket, Some(&self.credentials), delete_keys.iter());
            let signed_del = del_action.sign(Duration::from_secs(60));
            let (body, content_md5) = del_action.body_with_md5();

            tracing::debug!("Deleting a total of: {} keys", delete_keys.len());
            self.client
                .post(signed_del)
                .header("Content-MD5", content_md5)
                .body(body)
                .send()
                .await?
                .error_for_status()?;

            stop = parsed.start_after.is_none();
            last_key = parsed.start_after;

            tracing::debug!(
                "Deleted a total of: {} keys (last key? = {:?}, continue? = {})",
                delete_keys.len(),
                &last_key,
                !stop
            );
        }

        Ok(())
    }
}
