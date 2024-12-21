//! A collection of errors struct

/// A wrapper for [`FsError`].
pub type FsResult<T> = std::result::Result<T, FsError>;

/// A collection of the possible sources of the filesystem errors.
#[derive(Clone, Copy)]
pub enum FsErrorSource {
    /// A local disk error.
    Local,
    /// An S3-compatible error.
    S3,
}

impl std::fmt::Display for FsErrorSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsErrorSource::Local => write!(f, "Local"),
            FsErrorSource::S3 => write!(f, "S3"),
        }
    }
}

impl std::fmt::Debug for FsErrorSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsErrorSource::Local => write!(f, "FsErrorSource::Local"),
            FsErrorSource::S3 => write!(f, "FsErrorSource::S3"),
        }
    }
}

/// A collection of the filesystem errors.
pub struct FsError {
    /// The error message.
    message: String,
    /// The source of the error.
    source: FsErrorSource,
}

impl FsError {
    pub(crate) fn new(message: impl Into<String>, source: FsErrorSource) -> Self {
        Self {
            message: message.into(),
            source,
        }
    }
}

impl std::fmt::Display for FsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (source: {})", self.message, self.source)
    }
}

impl std::fmt::Debug for FsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (source: {})", self.message, self.source)
    }
}

impl std::error::Error for FsError {}

pub(crate) trait FsErrorExt {
    fn to_fserror(self, source: FsErrorSource) -> FsError;
}

impl FsErrorExt for std::io::Error {
    fn to_fserror(self, source: FsErrorSource) -> FsError {
        FsError::new(self.to_string(), source)
    }
}

impl FsErrorExt for reqwest::Error {
    fn to_fserror(self, source: FsErrorSource) -> FsError {
        FsError::new(self.to_string(), source)
    }
}

impl FsErrorExt for quick_xml::DeError {
    fn to_fserror(self, source: FsErrorSource) -> FsError {
        FsError::new(self.to_string(), source)
    }
}

/// Create a bail shortcut for FsError
#[macro_export(local_inner_macros)]
macro_rules! fs_bail {
    ($source:ident, $message:expr, $($arg:tt)*) => {
        return Err(crate::errors::FsError::new(std::format!($message, $($arg)*), crate::errors::FsErrorSource::$source))
    };
    ($source:ident, $message:expr) => {
        return Err(crate::errors::FsError::new($message, crate::errors::FsErrorSource::$source))
    }
}

/// Create a macro shortcut for FsError
#[macro_export(local_inner_macros)]
macro_rules! fs_error {
    ($source:ident, $message:expr, $($arg:tt)*) => {
        crate::errors::FsError::new(std::format!($message, $($arg)*), crate::errors::FsErrorSource::$source)
    };
    ($source:ident, $message:expr) => {
        crate::errors::FsError::new($message, crate::errors::FsErrorSource::$source)
    }
}
