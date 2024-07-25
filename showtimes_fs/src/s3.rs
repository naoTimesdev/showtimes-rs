use s3::{creds::Credentials, Bucket};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{make_file_path, FsFileKind, FsFileObject, FsImpl};

#[derive(Debug, Clone)]
pub struct S3Fs {
    bucket: Bucket,
}

impl S3Fs {
    /// Create a new instance of [`S3Fs`] filesystem implementation.
    ///
    /// # Parameters
    /// * `bucket`: The name of the bucket.
    /// * `access_key`: The access key for the bucket.
    /// * `secret_key`: The secret key for the bucket.
    /// * `region`: The region of the bucket.
    pub fn new(bucket: &str, access_key: &str, secret_key: &str, region: ::s3::Region) -> Self {
        let credentials =
            Credentials::new(Some(access_key), Some(secret_key), None, None, None).unwrap();
        let bucket = Bucket::new(bucket, region.clone(), credentials).unwrap();

        Self { bucket }
    }
}

#[async_trait::async_trait]
impl FsImpl for S3Fs {
    async fn init(&self) -> anyhow::Result<()> {
        // Test if the bucket exists
        self.bucket
            .list("/".to_string(), Some("/".to_string()))
            .await?;

        Ok(())
    }

    async fn file_stat(
        &self,
        base_key: &str,
        filename: &str,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<FsFileObject> {
        let key = make_file_path(base_key, filename, parent_id, kind);
        let (object, _) = self.bucket.head_object(&key).await?;

        let content_type = match object.content_type {
            Some(content_type) => content_type,
            None => {
                let guessed = mime_guess::from_path(filename);
                guessed.first_or_octet_stream().to_string()
            }
        };

        let last_modified = match object.last_modified {
            Some(last_modified) => {
                // last_modified is HTTP Last-Modified header
                match chrono::DateTime::parse_from_rfc2822(&last_modified) {
                    Ok(dt) => Some(dt.with_timezone(&chrono::Utc)),
                    Err(_) => None,
                }
            }
            None => None,
        };

        Ok(FsFileObject {
            filename: key,
            content_type,
            size: object.content_length.unwrap_or(-1),
            last_modified,
        })
    }

    async fn file_exists(
        &self,
        base_key: &str,
        filename: &str,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<bool> {
        let key = make_file_path(base_key, filename, parent_id, kind);
        let (object, _) = self.bucket.head_object(&key).await?;

        Ok(object.content_length.unwrap_or(-1) > 0)
    }

    async fn file_stream_upload<R: AsyncRead + Unpin + Send>(
        &self,
        base_key: &str,
        filename: &str,
        stream: &mut R,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<FsFileObject> {
        let key = make_file_path(base_key, filename, parent_id, kind.clone());
        self.bucket.put_object_stream(stream, &key).await?;

        self.file_stat(base_key, filename, parent_id, kind).await
    }

    async fn file_stream_download<W: AsyncWrite + Unpin + Send>(
        &self,
        base_key: &str,
        filename: &str,
        writer: &mut W,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<()> {
        let key = make_file_path(base_key, filename, parent_id, kind);
        self.bucket.get_object_to_writer(&key, writer).await?;

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
        self.bucket.delete_object(&key).await?;

        Ok(())
    }
}
