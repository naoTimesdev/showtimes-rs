//! A collection of errors data

use image::ImageError;

use crate::models::TMDbErrorResponse;

/// The wrapper for [`MetadataError`]
pub type MetadataResult<T> = Result<T, MetadataError>;

/// A global error for metadata problem
#[derive(Debug)]
pub enum MetadataError {
    /// Common error related to metadata
    CommonError(String),
    /// Error related to Anilist
    AnilistError,
    /// Error related to TMDb
    TMDbError(TMDbErrorResponse),
    /// Error related to VNDB
    VNDBError,
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
            Self::AnilistError => write!(f, "Anilist error"),
            Self::TMDbError(err) => write!(f, "TMDb error: {}", err),
            Self::VNDBError => write!(f, "VNDB error"),
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
        MetadataError::TMDbError(value)
    }
}

impl std::error::Error for MetadataError {}
impl std::error::Error for MetadataImageError {}
