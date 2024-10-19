#![warn(missing_docs, clippy::empty_docs, rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../README.md")]

use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

pub mod local;
pub mod s3;

/// The list of "pool" type of the filesystems.
///
/// Currently only `LocalFs` and `S3Fs` are supported.
pub enum FsPool {
    /// A local disk version
    LocalFs(crate::local::LocalFs),
    /// A S3-compatible version
    S3Fs(crate::s3::S3Fs),
}

/// The kind of the file system object.
#[derive(Default, Debug, Clone, tosho_macros::EnumName)]
pub enum FsFileKind {
    /// Images kind.
    #[default]
    Images,
    /// Invalid/fallback kind.
    Invalids,
}

impl FsFileKind {
    /// Convert the kind to a name used in the filesystem pathing.
    ///
    /// ```rust
    /// use showtimes_fs::FsFileKind;
    ///
    /// let kind = FsFileKind::Images;
    ///
    /// assert_eq!(kind.to_name(), "images");
    /// ```
    pub fn as_path_name(&self) -> String {
        self.to_name().to_ascii_lowercase()
    }
}

/// The file object in the filesystem.
#[derive(Debug, Clone)]
pub struct FsFileObject {
    /// The filename of the file.
    pub filename: String,
    /// The content type of the file.
    pub content_type: String,
    /// The size of the file.
    pub size: i64,
    /// The last modified time of the file.
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
}

/// Make a file path from the base key, filename, parent id, and kind.
pub(crate) fn make_file_path(
    base_key: &str,
    filename: &str,
    parent_id: Option<&str>,
    kind: Option<FsFileKind>,
) -> String {
    let kind = kind.unwrap_or_default().as_path_name();

    let mut path = format!("{}/", kind);
    if let Some(parent_id) = parent_id {
        path.push_str(&format!("{}/", parent_id.replace('-', "")))
    }
    path.push_str(&format!("{}/", base_key));
    path.push_str(filename);
    path
}

impl FsPool {
    /// Initialize the filesystem.
    ///
    /// This can be used to test if the filesystem is working correctly.
    pub async fn init(&self) -> anyhow::Result<()> {
        match self {
            Self::LocalFs(fs) => fs.init().await,
            Self::S3Fs(fs) => fs.init().await,
        }
    }
    /// Stat or get a file information in the filesystem.
    pub async fn file_stat(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        filename: impl Into<String> + std::marker::Send,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<FsFileObject> {
        match self {
            Self::LocalFs(fs) => fs.file_stat(base_key, filename, parent_id, kind).await,
            Self::S3Fs(fs) => fs.file_stat(base_key, filename, parent_id, kind).await,
        }
    }
    /// Check if a file exists in the filesystem.
    pub async fn file_exists(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        filename: impl Into<String> + std::marker::Send,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<bool> {
        match self {
            Self::LocalFs(fs) => fs.file_exists(base_key, filename, parent_id, kind).await,
            Self::S3Fs(fs) => fs.file_exists(base_key, filename, parent_id, kind).await,
        }
    }
    /// Upload a file to the filesystem.
    pub async fn file_stream_upload<R>(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        filename: impl Into<String> + std::marker::Send,
        mut stream: R,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<FsFileObject>
    where
        R: AsyncReadExt + AsyncSeekExt + Send + Unpin + 'static,
    {
        match self {
            Self::LocalFs(fs) => {
                fs.file_stream_upload(base_key, filename, &mut stream, parent_id, kind)
                    .await
            }
            Self::S3Fs(fs) => {
                fs.file_stream_upload(base_key, filename, &mut stream, parent_id, kind)
                    .await
            }
        }
    }
    /// Download a file from the filesystem.
    pub async fn file_stream_download<W>(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        filename: impl Into<String> + std::marker::Send,
        writer: &mut W,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<()>
    where
        W: AsyncWriteExt + Unpin + Send,
    {
        match self {
            Self::LocalFs(fs) => {
                fs.file_stream_download(base_key, filename, writer, parent_id, kind)
                    .await
            }
            Self::S3Fs(fs) => {
                fs.file_stream_download(base_key, filename, writer, parent_id, kind)
                    .await
            }
        }
    }
    /// Delete a file from the filesystem.
    pub async fn file_delete(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        filename: impl Into<String> + std::marker::Send,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<()> {
        match self {
            Self::LocalFs(fs) => fs.file_delete(base_key, filename, parent_id, kind).await,
            Self::S3Fs(fs) => fs.file_delete(base_key, filename, parent_id, kind).await,
        }
    }

    /// Delete a folder from the filesystem.
    pub async fn directory_delete(
        &self,
        base_key: impl Into<String> + std::marker::Send,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<()> {
        match self {
            Self::LocalFs(fs) => fs.directory_delete(base_key, parent_id, kind).await,
            Self::S3Fs(fs) => fs.directory_delete(base_key, parent_id, kind).await,
        }
    }

    /// Get the current filesystem name.
    pub fn get_name(&self) -> &'static str {
        match self {
            Self::LocalFs(_) => "Local",
            Self::S3Fs(_) => "S3",
        }
    }
}
