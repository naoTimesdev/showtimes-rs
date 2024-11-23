//! A collection of errors data

use image::ImageError;

use crate::models::{AnilistError, TMDbError, TMDbErrorResponse, VNDBError};

/// The wrapper for [`MetadataError`]
pub type MetadataResult<T> = Result<T, MetadataError>;

/// A global error for metadata problem
#[derive(Debug)]
pub enum MetadataError {
    /// Common error related to metadata
    CommonError(String),
    /// Error related to Anilist
    AnilistError(AnilistError),
    /// Error related to TMDb
    TMDbError(TMDbError),
    /// Error related to VNDB
    VNDBError(VNDBError),
    /// Error related to image processing
    ImageError(MetadataImageError),
}

/// An error related to image
#[derive(Debug)]
pub enum MetadataImageError {
    /// Failed to read image
    LoadError(ImageError),
    /// No dominant color found
    NoDominantColor,
    /// Invalid hex color string, the input is the string input
    InvalidHexColor(String),
}

impl std::fmt::Display for MetadataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommonError(e) => write!(f, "An error has occurred: {}", e),
            Self::AnilistError(err) => write!(f, "Anilist error: {}", err),
            Self::TMDbError(err) => write!(f, "TMDb error: {}", err),
            Self::VNDBError(err) => write!(f, "VNDB error: {}", err),
            Self::ImageError(e) => write!(f, "{}", e),
        }
    }
}

impl std::fmt::Display for MetadataImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoadError(e) => write!(f, "Failed to load image: {}", e),
            Self::NoDominantColor => write!(f, "No dominant color found on the image"),
            Self::InvalidHexColor(s) => write!(f, "Invalid hex color string: {}", s),
        }
    }
}

impl From<ImageError> for MetadataImageError {
    fn from(e: ImageError) -> Self {
        MetadataImageError::LoadError(e)
    }
}

impl From<MetadataImageError> for MetadataError {
    fn from(value: MetadataImageError) -> Self {
        MetadataError::ImageError(value)
    }
}

impl From<TMDbErrorResponse> for MetadataError {
    fn from(value: TMDbErrorResponse) -> Self {
        MetadataError::TMDbError(TMDbError::from(value))
    }
}

impl From<TMDbError> for MetadataError {
    fn from(value: TMDbError) -> Self {
        MetadataError::TMDbError(value)
    }
}

impl From<VNDBError> for MetadataError {
    fn from(value: VNDBError) -> Self {
        MetadataError::VNDBError(value)
    }
}

impl From<AnilistError> for MetadataError {
    fn from(value: AnilistError) -> Self {
        MetadataError::AnilistError(value)
    }
}

/// Error type that happens when parsing the response from the API
///
/// This is specifically for [`serde`] errors.
///
/// When formatted as a string, it will show the error message, status code, headers, and a JSON excerpt.
pub struct DetailedSerdeError {
    inner: serde_json::Error,
    status_code: reqwest::StatusCode,
    headers: reqwest::header::HeaderMap,
    url: reqwest::Url,
    raw_text: String,
}

impl std::fmt::Debug for DetailedSerdeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DetailedSerdeError")
            .field("inner", &self.inner)
            .field("status_code", &self.status_code)
            .field("headers", &self.headers)
            .field("url", &self.url)
            .field("excerpt", &self.get_json_excerpt())
            .finish()
    }
}

impl DetailedSerdeError {
    /// Create a new instance of the error
    pub(crate) fn new(
        inner: serde_json::Error,
        status_code: reqwest::StatusCode,
        headers: &reqwest::header::HeaderMap,
        url: &reqwest::Url,
        raw_text: impl Into<String>,
    ) -> Self {
        Self {
            inner,
            status_code,
            headers: headers.clone(),
            url: url.clone(),
            raw_text: raw_text.into(),
        }
    }

    /// Get the JSON excerpt from the raw text
    ///
    /// This will return a string that contains where the deserialization error happened.
    ///
    /// It will take 25 characters before and after the error position.
    pub fn get_json_excerpt(&self) -> String {
        let row_line = self.inner.line() - 1;
        let split_lines = self.raw_text.split('\n').collect::<Vec<&str>>();

        let position = self.inner.column();
        let start_idx = position.saturating_sub(25);
        let end_idx = position.saturating_add(25);

        // Bound the start and end index
        let start_idx = start_idx.max(0);
        let end_idx = end_idx.min(split_lines[row_line].len());

        split_lines[row_line][start_idx..end_idx].to_string()
    }
}

impl std::fmt::Display for DetailedSerdeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Serde Error: {}\nStatus Code: {}\nHeaders: {:?}\nURL: {}\nJSON excerpt: {}",
            self.inner,
            self.status_code,
            self.headers,
            self.url,
            self.get_json_excerpt()
        )
    }
}

impl std::error::Error for MetadataError {}
impl std::error::Error for MetadataImageError {}
