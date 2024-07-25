use tokio::io::{AsyncRead, AsyncWrite};

pub mod local;
pub mod s3;

pub use ::s3::Region;

/// The list of "pool" type of the filesystems.
///
/// Currently only `LocalFs` and `S3Fs` are supported.
pub enum FsPool {
    LocalFs(crate::local::LocalFs),
    S3Fs(crate::s3::S3Fs),
}

/// The kind of the file system object.
#[derive(Default, Debug, Clone, tosho_macros::EnumName)]
pub enum FsFileKind {
    #[default]
    Images,
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

/// Base trait for the filesystem implementations.
///
/// Implement some of the basic operations needed for it to work with Showtimes.
#[async_trait::async_trait]
pub trait FsImpl {
    /// Initialize the filesystem.
    ///
    /// This can be used to test if the filesystem is working correctly.
    async fn init(&self) -> anyhow::Result<()>;
    /// Stat or get a file information in the filesystem.
    async fn file_stat(
        &self,
        base_key: &str,
        filename: &str,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<FsFileObject>;
    /// Check if a file exists in the filesystem.
    async fn file_exists(
        &self,
        base_key: &str,
        filename: &str,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<bool>;
    /// Upload a file to the filesystem.
    async fn file_stream_upload<R: AsyncRead + Unpin + Send>(
        &self,
        base_key: &str,
        filename: &str,
        stream: &mut R,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<FsFileObject>;
    /// Download a file from the filesystem.
    async fn file_stream_download<W: AsyncWrite + Unpin + Send>(
        &self,
        base_key: &str,
        filename: &str,
        writer: &mut W,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<()>;
    /// Delete a file from the filesystem.
    async fn file_delete(
        &self,
        base_key: &str,
        filename: &str,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<()>;
}

/// Make a file path from the base key, filename, parent id, and kind.
pub(crate) fn make_file_path(
    base_key: &str,
    filename: &str,
    parent_id: Option<&str>,
    kind: Option<FsFileKind>,
) -> String {
    let kind = kind.unwrap_or_default().to_name().to_ascii_lowercase();

    let mut path = format!("{}/{}/", kind, base_key);
    if let Some(parent_id) = parent_id {
        path.push_str(&format!("{}/", parent_id.replace('-', "")))
    }
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
        base_key: &str,
        filename: &str,
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
        base_key: &str,
        filename: &str,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<bool> {
        match self {
            Self::LocalFs(fs) => fs.file_exists(base_key, filename, parent_id, kind).await,
            Self::S3Fs(fs) => fs.file_exists(base_key, filename, parent_id, kind).await,
        }
    }
    /// Upload a file to the filesystem.
    pub async fn file_stream_upload<R: AsyncRead + Unpin + Send>(
        &self,
        base_key: &str,
        filename: &str,
        stream: &mut R,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<FsFileObject> {
        match self {
            Self::LocalFs(fs) => {
                fs.file_stream_upload(base_key, filename, stream, parent_id, kind)
                    .await
            }
            Self::S3Fs(fs) => {
                fs.file_stream_upload(base_key, filename, stream, parent_id, kind)
                    .await
            }
        }
    }
    /// Download a file from the filesystem.
    pub async fn file_stream_download<W: AsyncWrite + Unpin + Send>(
        &self,
        base_key: &str,
        filename: &str,
        writer: &mut W,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<()> {
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
        base_key: &str,
        filename: &str,
        parent_id: Option<&str>,
        kind: Option<FsFileKind>,
    ) -> anyhow::Result<()> {
        match self {
            Self::LocalFs(fs) => fs.file_delete(base_key, filename, parent_id, kind).await,
            Self::S3Fs(fs) => fs.file_delete(base_key, filename, parent_id, kind).await,
        }
    }
}
