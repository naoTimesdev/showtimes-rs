//! A local disk client for accessing filesystem.

use std::path::PathBuf;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
    FsFileKind, FsFileObject,
    errors::{FsErrorExt, FsErrorSource, FsResult},
    fs_bail, fs_error, make_file_path,
};

/// A local disk client for accessing filesystem.
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

    pub(crate) async fn init(&self) -> FsResult<()> {
        // Test if the directory exists
        tracing::debug!(
            "Initializing, checking if the directory exists: {:?}",
            &self.directory
        );
        let item = tokio::fs::metadata(&self.directory)
            .await
            .map_err(|e| e.to_fserror(FsErrorSource::Local))?;
        if !item.is_dir() {
            fs_bail!(Local, "The provided `directory` is not a directory");
        }

        Ok(())
    }

    pub(crate) async fn file_stat(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        filename: impl Into<String> + std::marker::Send,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> FsResult<FsFileObject> {
        let key = make_file_path(&base_key.into(), &filename.into(), parent_id, kind);
        let path = self.directory.join(&key);

        tracing::debug!("Checking file stat for: {}", &key);
        let item = tokio::fs::metadata(&path)
            .await
            .map_err(|e| e.to_fserror(FsErrorSource::Local))?;
        let content_type = mime_guess::from_path(path).first_or_octet_stream();
        let last_modified = match item.modified() {
            Ok(time) => jiff::Timestamp::try_from(time).ok(),
            Err(_) => None,
        };

        let fs_meta = FsFileObject {
            filename: key.clone(),
            content_type: content_type.to_string(),
            size: item.len().try_into().map_err(|_| {
                fs_error!(Local, "Failed to convert file size to i64: {}", item.len())
            })?,
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
    ) -> FsResult<bool> {
        let key = make_file_path(&base_key.into(), &filename.into(), parent_id, kind.clone());
        let path = self.directory.join(&key);

        tracing::debug!("Checking file existence for: {}", &key);
        let is_exists = (tokio::fs::try_exists(&path).await).unwrap_or(false);

        Ok(is_exists)
    }

    pub(crate) async fn file_stream_upload<R>(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        filename: impl Into<String> + std::marker::Send,
        stream: &mut R,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> FsResult<FsFileObject>
    where
        R: AsyncReadExt + Send + Unpin + 'static,
    {
        let base_key: String = base_key.into();
        let filename: String = filename.into();
        let key = make_file_path(&base_key, &filename, parent_id, kind.clone());
        let path = self.directory.join(&key);

        tracing::debug!("Sending file stream into: {}", &key);
        let mut file = tokio::fs::File::create(&path)
            .await
            .map_err(|e| e.to_fserror(FsErrorSource::Local))?;
        tokio::io::copy(stream, &mut file)
            .await
            .map_err(|e| e.to_fserror(FsErrorSource::Local))?;

        self.file_stat(base_key, filename, parent_id, kind).await
    }

    pub(crate) async fn file_stream_download<W>(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        filename: impl Into<String> + std::marker::Send,
        writer: &mut W,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> FsResult<()>
    where
        W: AsyncWriteExt + Unpin + Send,
    {
        let key = make_file_path(&base_key.into(), &filename.into(), parent_id, kind.clone());
        let path = self.directory.join(&key);

        tracing::debug!("Downloading file stream for: {}", &key);
        let mut file = tokio::fs::File::open(&path)
            .await
            .map_err(|e| e.to_fserror(FsErrorSource::Local))?;
        tokio::io::copy(&mut file, writer)
            .await
            .map_err(|e| e.to_fserror(FsErrorSource::Local))?;

        tracing::debug!("Download complete for: {}", &key);
        writer
            .flush()
            .await
            .map_err(|e| e.to_fserror(FsErrorSource::Local))?;

        Ok(())
    }

    pub(crate) async fn file_delete(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        filename: impl Into<String> + std::marker::Send,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> FsResult<()> {
        let key = make_file_path(&base_key.into(), &filename.into(), parent_id, kind.clone());
        let path = self.directory.join(&key);

        tracing::debug!("Deleting file: {}", &key);
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| e.to_fserror(FsErrorSource::Local))?;

        Ok(())
    }

    pub(crate) async fn directory_delete(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> FsResult<()> {
        let key = make_file_path(&base_key.into(), "", parent_id, kind);
        let path = self.directory.join(&key);

        tracing::debug!("Deleting directory: {}", &key);
        tokio::fs::remove_dir_all(&path)
            .await
            .map_err(|e| e.to_fserror(FsErrorSource::Local))?;

        Ok(())
    }
}
