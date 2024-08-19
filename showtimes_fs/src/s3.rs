use std::sync::Arc;

use crate::{make_file_path, FsFileKind, FsFileObject, FsImpl};
use aws_config::AppName;
use aws_credential_types::provider::{self, error::CredentialsError, future, ProvideCredentials};
use aws_credential_types::Credentials;
use aws_sdk_s3::primitives::{ByteStream, DateTimeFormat, SdkBody};
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct S3FsCredentialsProvider {
    access_key: String,
    secret_key: String,
}

impl S3FsCredentialsProvider {
    pub fn new(access_key: &str, secret_key: &str) -> Self {
        Self {
            access_key: access_key.to_string(),
            secret_key: secret_key.to_string(),
        }
    }

    fn credentials(&self) -> provider::Result {
        if self.access_key.is_empty() {
            return Err(CredentialsError::not_loaded("Access key is empty"));
        }
        if self.secret_key.is_empty() {
            return Err(CredentialsError::not_loaded("Secret key is empty"));
        };

        Ok(Credentials::new(
            self.access_key.clone(),
            self.secret_key.clone(),
            None,
            None,
            ENV_PROVIDER,
        ))
    }
}

const ENV_PROVIDER: &str = "S3FsCredentialsProvider";

impl ProvideCredentials for S3FsCredentialsProvider {
    fn provide_credentials<'a>(&'a self) -> future::ProvideCredentials<'a>
    where
        Self: 'a,
    {
        future::ProvideCredentials::ready(self.credentials())
    }
}

#[derive(Debug, Clone)]
pub struct S3FsRegionProvider {
    region: String,
    endpoint_url: Option<String>,
}

impl S3FsRegionProvider {
    pub fn new(region: &str, endpoint_url: Option<&str>) -> Self {
        Self {
            region: region.to_string(),
            endpoint_url: endpoint_url.map(|s| s.to_string()),
        }
    }

    pub fn region(&self) -> &str {
        &self.region
    }

    pub fn endpoint_url(&self) -> Option<&str> {
        self.endpoint_url.as_deref()
    }
}

#[derive(Debug, Clone)]
pub struct S3Fs {
    bucket_name: String,
    client: Arc<Mutex<aws_sdk_s3::Client>>,
}

impl S3Fs {
    /// Create a new instance of [`S3Fs`] filesystem implementation.
    ///
    /// # Parameters
    /// * `bucket`: The name of the bucket.
    /// * `credentials`: The credentials provider.
    /// * `region`: The region of the bucket.
    pub async fn new(
        bucket: &str,
        credentials: S3FsCredentialsProvider,
        region: S3FsRegionProvider,
    ) -> Self {
        // Test if the bucket exists
        let config_loader = aws_config::from_env()
            .app_name(AppName::new("showtimes-fs-rs").unwrap())
            .region(aws_types::region::Region::new(region.region))
            .credentials_provider(credentials);
        let config_loader = match &region.endpoint_url {
            Some(endpoint_url) => config_loader.endpoint_url(endpoint_url),
            None => config_loader,
        };

        let config = config_loader.load().await;

        tracing::debug!("Creating S3Fs with config: {:?}", config);
        let client = aws_sdk_s3::Client::new(&config);

        Self {
            bucket_name: bucket.to_string(),
            client: Arc::new(Mutex::new(client)),
        }
    }
}

struct CustomS3Writer {
    temp_bytes: bytes::BytesMut,
}

impl CustomS3Writer {
    fn new() -> Self {
        Self {
            temp_bytes: bytes::BytesMut::new(),
        }
    }

    fn as_bytes(&self) -> bytes::Bytes {
        self.temp_bytes.clone().freeze()
    }
}

impl AsyncWrite for CustomS3Writer {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let this = std::pin::Pin::into_inner(self);
        let len = buf.len();
        this.temp_bytes.extend_from_slice(buf);
        std::task::Poll::Ready(Ok(len))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
}

#[async_trait::async_trait]
impl FsImpl for S3Fs {
    async fn init(&self) -> anyhow::Result<()> {
        let bucket = self.bucket_name.clone();
        let client = self.client.lock().await;
        tracing::debug!("Initializing, checking if bucket {} exists...", bucket);
        let buckets = client.list_buckets().send().await?;

        let matched = buckets
            .buckets()
            .iter()
            .find(|&b| b.name() == Some(&bucket));
        tracing::debug!("Bucket {} match into: {:?}", bucket, matched);
        match matched {
            Some(_) => Ok(()),
            None => {
                // Create the bucket
                client.create_bucket().bucket(&bucket).send().await?;
                Ok(())
            }
        }
    }

    async fn file_stat(
        &self,
        base_key: &str,
        filename: &str,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<FsFileObject> {
        let key = make_file_path(base_key, filename, parent_id, kind);
        let client = self.client.lock().await;
        tracing::debug!("Checking file stat for: {}", &key);
        let object = client
            .head_object()
            .bucket(&self.bucket_name)
            .key(&key)
            .send()
            .await?;

        let content_type = match object.content_type {
            Some(content_type) => {
                // Check if octet-stream
                if content_type.to_ascii_lowercase().contains("/octet-stream") {
                    let guessed = mime_guess::from_path(filename);
                    guessed.first_or_octet_stream().to_string()
                } else {
                    content_type
                }
            }
            None => {
                let guessed = mime_guess::from_path(filename);
                guessed.first_or_octet_stream().to_string()
            }
        };

        let last_mod = match object.last_modified {
            Some(last_mod) => match last_mod.fmt(DateTimeFormat::DateTime) {
                Ok(s) => match chrono::DateTime::parse_from_rfc3339(&s) {
                    Ok(dt) => Some(dt.with_timezone(&chrono::Utc)),
                    Err(_) => None,
                },
                Err(_) => None,
            },
            None => None,
        };

        let fs_meta = FsFileObject {
            filename: key.clone(),
            content_type,
            size: object.content_length.unwrap_or(-1),
            last_modified: last_mod,
        };

        tracing::debug!("File stat for {}: {:?}", &key, fs_meta);

        Ok(fs_meta)
    }

    async fn file_exists(
        &self,
        base_key: &str,
        filename: &str,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<bool> {
        let key = make_file_path(base_key, filename, parent_id, kind);
        let client = self.client.lock().await;
        tracing::debug!("Checking file existence for: {}", &key);
        let object = client
            .head_object()
            .bucket(&self.bucket_name)
            .key(&key)
            .send()
            .await?;

        Ok(object.content_length.unwrap_or(-1) > 0)
    }

    async fn file_stream_upload<R: AsyncReadExt + Unpin + Send>(
        &self,
        base_key: &str,
        filename: &str,
        stream: &mut R,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<FsFileObject> {
        let key = make_file_path(base_key, filename, parent_id, kind.clone());
        let client = self.client.lock().await;
        // Create a temporary writer since AWS SDK s3 FUCKING SUCKS!
        tracing::debug!("Initializing file upload to: {}", &key);
        let mut target = CustomS3Writer::new();
        tokio::io::copy(stream, &mut target).await?;
        let body = ByteStream::new(SdkBody::from(target.as_bytes()));

        tracing::debug!("Sending file stream into: {}", &key);
        client
            .put_object()
            .bucket(&self.bucket_name)
            .key(&key)
            .body(body)
            .send()
            .await?;

        // unlock so we can do file stat
        tracing::debug!("Upload complete, unlocking Mutex guard for: {}", &key);
        std::mem::drop(client);

        self.file_stat(base_key, filename, parent_id, kind).await
    }

    async fn file_stream_download<W: AsyncWriteExt + Unpin + Send>(
        &self,
        base_key: &str,
        filename: &str,
        writer: &mut W,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<()> {
        let key = make_file_path(base_key, filename, parent_id, kind);
        let client = self.client.lock().await;
        tracing::debug!("Initializing file download for: {}", &key);
        let mut resp = client
            .get_object()
            .bucket(&self.bucket_name)
            .key(&key)
            .send()
            .await?;

        tracing::debug!("Downloading file stream for: {}", &key);
        while let Some(bytes) = resp.body.try_next().await? {
            writer.write_all(&bytes).await?;
        }

        Ok(())
    }

    async fn file_delete(
        &self,
        base_key: &str,
        filename: &str,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<()> {
        let key = make_file_path(base_key, filename, parent_id, kind);
        let client = self.client.lock().await;
        tracing::debug!("Deleting file: {}", &key);
        client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(&key)
            .send()
            .await?;

        Ok(())
    }

    async fn directory_delete(
        &self,
        base_key: &str,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<()> {
        let client = self.client.lock().await;
        let prefix = make_file_path(base_key, "", parent_id, kind);

        tracing::debug!("Collecting objects with prefix: {}", &prefix);
        let mut pages = client
            .list_objects_v2()
            .bucket(&self.bucket_name)
            .prefix(&prefix)
            .into_paginator()
            .send();

        let mut delete_objects: Vec<ObjectIdentifier> = Vec::new();
        while let Some(page) = pages.next().await {
            let page = page?;
            if let Some(content) = page.contents {
                let objects: Vec<ObjectIdentifier> = content
                    .iter()
                    .filter_map(|c| {
                        c.key().map(|key| {
                            ObjectIdentifier::builder()
                                .set_key(Some(key.to_string()))
                                .build()
                                .unwrap()
                        })
                    })
                    .collect();

                tracing::debug!("Adding {} objects for deletion", objects.len());

                delete_objects.extend(objects);
            }
        }

        tracing::debug!("Deleting {} objects", delete_objects.len());
        let delete_in = Delete::builder()
            .set_objects(Some(delete_objects))
            .build()?;

        client
            .delete_objects()
            .bucket(&self.bucket_name)
            .delete(delete_in)
            .send()
            .await?;

        tracing::debug!("Deletion complete");
        Ok(())
    }
}
