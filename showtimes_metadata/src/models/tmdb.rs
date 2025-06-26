//! A type definition for the TMDb API
//!
//! This is incomplete and only made to support what Showtimes needed.

use serde::{Deserialize, Serialize};

use crate::errors::DetailedSerdeError;

const BASE_IMAGE: &str = "https://image.tmdb.org/t/p/";

/// The media type of the TMDb API
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TMDbMediaType {
    /// A movie
    Movie,
    /// A TV series
    Tv,
    /// A collection of movies
    Collection,
    /// A company
    Company,
    /// A keyword
    Keyword,
    /// A person
    Person,
}

/// The sizes of the poster images of the TMDb API
///
/// Usually 2:3 aspect ratio
#[derive(Debug, Clone, Copy, PartialEq, showtimes_derive::EnumName)]
pub enum TMDbPosterSize {
    /// 92x138
    W92,
    /// 154x231
    W154,
    /// 185x278
    W185,
    /// 342x513
    W342,
    /// 500x750
    W500,
    /// 780x1170
    W780,
    /// The original size
    Original,
}

/// The sizes of the backdrop images of the TMDb API
///
/// Usually 16:9 aspect ratio
#[derive(Debug, Clone, Copy, PartialEq, showtimes_derive::EnumName)]
pub enum TMDbBackdropSize {
    /// 300x169
    W300,
    /// 780x438
    W780,
    /// 1280x720
    W1280,
    /// The original size
    Original,
}

/// A single result from the TMDb API about a movie
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct TMDbMovieResult {
    /// The ID of the result
    pub id: i32,
    /// Is this an adult/NSFW item
    pub adult: bool,
    /// The title of the result.
    pub title: Option<String>,
    /// The original title of the result.
    pub original_title: Option<String>,
    /// The original language of the result
    pub original_language: Option<String>,
    /// The poster path of the result
    ///
    /// This is not a full URL, but need to be appended with some base URL.
    ///
    /// See [TMDb Image API](https://developers.themoviedb.org/3/getting-started/images) for more information
    pub poster_path: Option<String>,
    /// The backdrop path of the result
    ///
    /// Same with `poster_path`, this is not a full URL, but need to be appended with some base URL.
    pub backdrop_path: Option<String>,
    /// The release date of the result
    pub release_date: Option<String>,
    /// Overview/description of the result
    pub overview: Option<String>,
}

impl TMDbMovieResult {
    /// Get the full URL of the poster path.
    ///
    /// This will utilize the `original` size of the image and will
    /// return `None` if the `poster_path` is `None`.
    pub fn poster_url(&self) -> Option<String> {
        self.poster_url_sized(TMDbPosterSize::Original)
    }

    /// Get the full URL of the poster path with a specific size.
    ///
    /// This will return `None` if the `poster_path` is `None`.
    pub fn poster_url_sized(&self, size: TMDbPosterSize) -> Option<String> {
        self.poster_path.as_ref().map(|path| get_poster(path, size))
    }

    /// Get the full URL of the backdrop path.
    ///
    /// This will utilize the `original` size of the image and will
    /// return `None` if the `backdrop_path` is `None`.
    pub fn backdrop_url(&self) -> Option<String> {
        self.backdrop_url_sized(TMDbBackdropSize::Original)
    }

    /// Get the full URL of the backdrop path with a specific size.
    ///
    /// This will return `None` if the `backdrop_path` is `None`.
    pub fn backdrop_url_sized(&self, size: TMDbBackdropSize) -> Option<String> {
        self.backdrop_path
            .as_ref()
            .map(|path| get_backdrop(path, size))
    }
}

/// A multi result from the TMDb API
///
/// This covers almost all the possible search result of each media types.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct TMDbMultiResult {
    /// The ID of the result
    pub id: i32,
    /// Is this an adult/NSFW item
    pub adult: bool,
    /// The media type of the result
    pub media_type: TMDbMediaType,
    /// The name of the result.
    /// Used in `TV`, `Collection`, `Company`, `Keyword`, and `Person`
    pub name: Option<String>,
    /// The original name of the result.
    /// Used in `TV`, `Collection`, `Company`, `Keyword`, and `Person`
    pub original_name: Option<String>,
    /// The title of the result.
    /// Used in `Movie`
    pub title: Option<String>,
    /// The original title of the result.
    /// Used in `Movie`
    pub original_title: Option<String>,
    /// The original language of the result
    pub original_language: Option<String>,
    /// The poster path of the result
    ///
    /// This is not a full URL, but need to be appended with some base URL.
    ///
    /// See [TMDb Image API](https://developers.themoviedb.org/3/getting-started/images) for more information
    pub poster_path: Option<String>,
    /// The backdrop path of the result
    ///
    /// Same with `poster_path`, this is not a full URL, but need to be appended with some base URL.
    pub backdrop_path: Option<String>,
    /// The release date of the result
    pub release_date: Option<String>,
    /// The first air date of the result (for TV)
    ///
    /// This is YYYY-MM-DD, but can be YYYY-MM or YYYY
    pub first_air_date: Option<String>,
    /// Overview/description of the result
    pub overview: Option<String>,
}

impl TMDbMultiResult {
    /// Get the full URL of the poster path.
    ///
    /// This will utilize the `original` size of the image and will
    /// return `None` if the `poster_path` is `None`.
    pub fn poster_url(&self) -> Option<String> {
        self.poster_url_sized(TMDbPosterSize::Original)
    }

    /// Get the full URL of the poster path with a specific size.
    ///
    /// This will return `None` if the `poster_path` is `None`.
    pub fn poster_url_sized(&self, size: TMDbPosterSize) -> Option<String> {
        self.poster_path.as_ref().map(|path| get_poster(path, size))
    }

    /// Get the full URL of the backdrop path.
    ///
    /// This will utilize the `original` size of the image and will
    /// return `None` if the `backdrop_path` is `None`.
    pub fn backdrop_url(&self) -> Option<String> {
        self.backdrop_url_sized(TMDbBackdropSize::Original)
    }

    /// Get the full URL of the backdrop path with a specific size.
    ///
    /// This will return `None` if the `backdrop_path` is `None`.
    pub fn backdrop_url_sized(&self, size: TMDbBackdropSize) -> Option<String> {
        self.backdrop_path
            .as_ref()
            .map(|path| get_backdrop(path, size))
    }

    /// Get the title of the result.
    ///
    /// This will check if the `title` is `None` and will return the `name` if it is.
    pub fn title(&self) -> String {
        self.title
            .as_ref()
            .map(|title| title.to_string())
            .unwrap_or_else(|| self.name.as_ref().unwrap().to_string())
    }

    /// Get the original title of the result.
    ///
    /// This will check if the `original_title` is `None` and will return the `original_name` if it is.
    pub fn original_title(&self) -> Option<String> {
        self.original_title
            .as_ref()
            .map(|title| title.to_string())
            .or_else(|| self.original_name.as_ref().map(|name| name.to_string()))
    }

    /// Get the release date of the result.
    ///
    /// This will check if the `release_date` is `None` and will return the `first_air_date` if it is.
    pub fn release_date(&self) -> Option<String> {
        self.release_date
            .as_ref()
            .map(|date| date.to_string())
            .or_else(|| self.first_air_date.as_ref().map(|date| date.to_string()))
    }
}

/// Response from TMDb API with multiple results
///
/// This is used for APIs that return multiple results such as search
/// and discover.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct TMDbMultiResponse<T> {
    /// The page of the response
    pub page: u32,
    /// The total number of pages
    pub total_pages: u32,
    /// The total number of results
    pub total_results: u32,
    /// The results of the response
    #[serde(bound(deserialize = "T: Deserialize<'de>", serialize = "T: Serialize"))]
    pub results: Vec<T>,
}

/// Error response from TMDb API
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct TMDbErrorResponse {
    /// The status code of the error
    pub status_code: i32,
    /// The status message of the error
    pub status_message: String,
}

impl std::fmt::Display for TMDbErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (code: {})", self.status_message, self.status_code)
    }
}

impl std::error::Error for TMDbErrorResponse {}

/// Error type for TMDb API
///
/// This enum can be used to wrap the possible errors that can happen when
/// interacting with the TMDb API.
#[derive(Debug)]
pub enum TMDbError {
    /// Error related to response result
    Response(TMDbErrorResponse),
    /// Error related to request
    Request(reqwest::Error),
    /// Error related to deserialization
    Serde(Box<DetailedSerdeError>),
}

impl TMDbError {
    pub(crate) fn new_serde(err: DetailedSerdeError) -> Self {
        TMDbError::Serde(Box::new(err))
    }
}

impl From<DetailedSerdeError> for TMDbError {
    fn from(value: DetailedSerdeError) -> Self {
        TMDbError::Serde(Box::new(value))
    }
}

impl From<reqwest::Error> for TMDbError {
    fn from(value: reqwest::Error) -> Self {
        TMDbError::Request(value)
    }
}

impl From<TMDbErrorResponse> for TMDbError {
    fn from(value: TMDbErrorResponse) -> Self {
        TMDbError::Response(value)
    }
}

impl std::fmt::Display for TMDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TMDbError::Response(err) => write!(f, "{err}"),
            TMDbError::Request(err) => write!(f, "{err}"),
            TMDbError::Serde(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for TMDbError {}

fn get_backdrop(url: &str, size: TMDbBackdropSize) -> String {
    format!("{}{}{}", BASE_IMAGE, size.to_name().to_lowercase(), url)
}

fn get_poster(url: &str, size: TMDbPosterSize) -> String {
    format!("{}{}{}", BASE_IMAGE, size.to_name().to_lowercase(), url)
}
