use std::path::PathBuf;

use chrono::{DateTime, Utc};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{make_file_path, FsFileKind, FsFileObject, FsImpl};

#[derive(Debug, Clone)]
pub struct LocalFs {
    directory: PathBuf,
}

impl LocalFs {
    /// Create a new instance of [`LocalFs`] filesystem implementation.
    ///
    /// # Parameters
    /// * `bucket`: The name of the bucket.
    /// * `access_key`: The access key for the bucket.
    /// * `secret_key`: The secret key for the bucket.
    /// * `region`: The region of the bucket.
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }
}

#[async_trait::async_trait]
impl FsImpl for LocalFs {
    async fn init(&self) -> anyhow::Result<()> {
        // Test if the directory exists
        tracing::debug!(
            "Initializing, checking if the directory exists: {:?}",
            &self.directory
        );
        let item = tokio::fs::metadata(&self.directory).await?;
        if !item.is_dir() {
            anyhow::bail!("The provided `directory` is not a directory");
        }

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
        let path = self.directory.join(&key);

        tracing::debug!("Checking file stat for: {}", &key);
        let item = tokio::fs::metadata(&path).await?;
        let content_type = mime_guess::from_path(path).first_or_octet_stream();
        let last_modified = match item.modified() {
            Ok(time) => {
                let dt: DateTime<Utc> = time.into();
                Some(dt)
            }
            Err(_) => None,
        };

        let fs_meta = FsFileObject {
            filename: key.clone(),
            content_type: content_type.to_string(),
            size: item.len().try_into().unwrap(),
            last_modified,
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
        let path = self.directory.join(&key);

        tracing::debug!("Checking file existence for: {}", &key);
        let is_exists = (tokio::fs::try_exists(&path).await).unwrap_or(false);

        Ok(is_exists)
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
        let path = self.directory.join(&key);

        tracing::debug!("Sending file stream into: {}", &key);
        let mut file = tokio::fs::File::create(&path).await?;
        tokio::io::copy(stream, &mut file).await?;

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
        let path = self.directory.join(&key);

        tracing::debug!("Downloading file stream for: {}", &key);
        let mut file = tokio::fs::File::open(&path).await?;
        tokio::io::copy(&mut file, writer).await?;

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
        let path = self.directory.join(&key);

        tracing::debug!("Deleting file: {}", &key);
        tokio::fs::remove_file(&path).await?;

        Ok(())
    }
}
