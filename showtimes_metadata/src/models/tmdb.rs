use serde::{Deserialize, Serialize};

const BASE_IMAGE: &str = "https://image.tmdb.org/t/p/";

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TMDbMediaType {
    Movie,
    Tv,
    Collection,
    Company,
    Keyword,
    Person,
}

#[derive(Debug, Clone, Copy, PartialEq, tosho_macros::EnumName)]
pub enum TMDbPosterSize {
    W92,
    W154,
    W185,
    W342,
    W500,
    W780,
    Original,
}

#[derive(Debug, Clone, Copy, PartialEq, tosho_macros::EnumName)]
pub enum TMDbBackdropSize {
    W300,
    W780,
    W1280,
    Original,
}

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
        let downcast = size.to_name().to_lowercase();

        self.poster_path
            .as_ref()
            .map(|path| format!("{}{}{}", BASE_IMAGE, downcast, path))
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
        let downcast = size.to_name().to_lowercase();

        self.backdrop_path
            .as_ref()
            .map(|path| format!("{}{}{}", BASE_IMAGE, downcast, path))
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct TMDbMultiResponse {
    /// The page of the response
    pub page: u32,
    /// The total number of pages
    pub total_pages: u32,
    /// The total number of results
    pub total_results: u32,
    /// The results of the response
    pub results: Vec<TMDbMultiResult>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct TMDbErrorResponse {
    /// The status code of the error
    pub status_code: i32,
    /// The status message of the error
    pub status_message: String,
}

impl std::fmt::Display for TMDbErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TMDb Error {}: {}",
            self.status_code, self.status_message
        )
    }
}
