//! A type definition for the Anilist API
//!
//! This is incomplete and only made to support what Showtimes needed.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::{errors::DetailedSerdeError, image::hex_to_u32};

static JST_TZ: LazyLock<jiff::tz::TimeZone> =
    LazyLock::new(|| jiff::tz::TimeZone::get("Asia/Tokyo").unwrap());

/// Media type
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnilistMediaType {
    /// Anime media type
    Anime,
    /// Manga media type
    Manga,
}

/// Media format
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnilistMediaFormat {
    /// TV media format
    Tv,
    /// Short-TV media format, usually around 3-12 minutes long compared to the usual 20+ minutes runtime
    TvShort,
    /// Movie media format
    Movie,
    /// Special media format, mostly used as a one-shot or one-off event
    Special,
    /// Original Video Animation, commonly used for Anime bonuses on DVDs and Blu-Rays
    #[serde(rename = "OVA")]
    OVA,
    /// Original Net(work) Animation, used sometimes for Anime that only streams online on platform like Amazon or Netflix
    #[serde(rename = "ONA")]
    ONA,
    /// Music video media format
    Music,
    /// Manga media format
    Manga,
    /// Novel or Light Novel media format
    Novel,
    /// One-shot media format, used on Manga and Novel media
    OneShot,
}

/// A "fuzzy"-date where we might not have all the information
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
pub struct AnilistFuzzyDate {
    /// The year
    pub year: Option<i16>,
    /// The month
    pub month: Option<i8>,
    /// The day
    pub day: Option<i8>,
}

impl AnilistFuzzyDate {
    /// Parse the fuzzy date into a [`jiff::Timestamp`]
    pub fn into_timestamp(&self) -> Option<jiff::Timestamp> {
        match (self.year, self.month, self.day) {
            (Some(year), Some(month), Some(day)) => jiff::civil::Date::new(year, month, day)
                .and_then(|date| date.to_zoned(JST_TZ.clone()))
                .map(|dt| dt.timestamp())
                .ok(),
            _ => None,
        }
    }
}

impl std::fmt::Display for AnilistFuzzyDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // YYYY-MM-DD, YYYY-MM, or YYYY, or ""
        write!(
            f,
            "{}{}{}",
            self.year.map_or(String::new(), |y| y.to_string()),
            self.month.map_or(String::new(), |m| format!("-{m:02}")),
            self.day.map_or(String::new(), |d| format!("-{d:02}")),
        )
    }
}

/// Airing schedule of the Anime
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
pub struct AnilistAiringSchedule {
    /// The ID of the airing schedule
    pub id: i32,
    /// The episode of the anime
    pub episode: i32,
    /// The time the episode airs at
    #[serde(rename = "airingAt")]
    pub airing_at: i64,
}

/// The collection of cover image from larger to smaller
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct AnilistCoverImage {
    /// The smallest cover image, usually around 100x150
    pub medium: Option<String>,
    /// The medium cover image, usually around 230x350
    pub large: Option<String>,
    /// The largest cover image, usually around 425x640
    #[serde(rename = "extraLarge")]
    pub extra_large: Option<String>,
    /// Average hex color of the image
    color: Option<String>,
}

impl AnilistCoverImage {
    /// Get the largest cover image
    pub fn get_image(&self) -> Option<String> {
        self.extra_large
            .clone()
            .or_else(|| self.large.clone())
            .or_else(|| self.medium.clone())
    }

    /// Get the average color of the image as a u32 from the hex color.
    ///
    /// `None` is returned if the color is not available or could not be parsed.
    pub fn get_color(&self) -> Option<u32> {
        self.color.as_ref().and_then(|color| hex_to_u32(color).ok())
    }
}

/// The title of the media in different languages
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct AnilistMediaTitle {
    /// The romaji title of the media
    pub romaji: Option<String>,
    /// The english title of the media
    pub english: Option<String>,
    /// The native title of the media
    pub native: Option<String>,
}

/// The result of a media schedule search
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct AnilistMediaScheduleResult {
    /// The airing schedule of the media
    pub airing_schedule: AnilistAiringSchedule,
}

/// The media result from the Anilist API
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct AnilistMedia {
    /// The ID of the media
    pub id: i32,
    /// The ID of the media on MyAnimeList
    #[serde(rename = "idMal")]
    pub id_mal: Option<i32>,
    /// The title of the media in different languages
    pub title: AnilistMediaTitle,
    /// The cover image of the media in different sizes
    #[serde(rename = "coverImage")]
    pub cover_image: AnilistCoverImage,
    /// The release date of the media
    #[serde(rename = "startDate")]
    pub start_date: Option<AnilistFuzzyDate>,
    /// The season of the media
    pub season: Option<String>,
    /// The year of the media
    #[serde(rename = "seasonYear")]
    pub season_year: Option<i32>,
    /// The number of episodes of the media
    pub episodes: Option<i32>,
    /// The number of chapters of the media
    pub chapters: Option<i32>,
    /// The number of volumes of the media
    pub volumes: Option<i32>,
    /// The format of the media
    pub format: AnilistMediaFormat,
    /// The type of the media
    #[serde(rename = "type")]
    pub kind: AnilistMediaType,
    /// The description of the media
    pub description: Option<String>,
    /// If the media is for adult or not
    #[serde(rename = "isAdult")]
    pub is_adult: bool,
}

/// The information of the page
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnilistPageInfo {
    /// The total of the results
    pub total: i32,
    /// The results per page
    pub per_page: i32,
    /// The current page
    pub current_page: i32,
    /// If there is a next page or not
    pub has_next_page: bool,
}

/// The nodes of a paginated result of the Anilist API.
///
/// Use the function provided to get each node.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum AnilistPaginatedNodes {
    /// The airing schedules result
    AiringSchedules {
        /// The airing schedules
        #[serde(rename = "airingSchedules")]
        airing_schedules: Vec<AnilistAiringSchedule>,
    },
    /// The media result
    Media {
        /// The media
        media: Vec<AnilistMedia>,
    },
}

impl AnilistPaginatedNodes {
    /// Get the airing schedules result from the paginated nodes
    pub fn airing_schedules(&self) -> Option<&Vec<AnilistAiringSchedule>> {
        match self {
            AnilistPaginatedNodes::AiringSchedules { airing_schedules } => Some(airing_schedules),
            _ => None,
        }
    }

    /// Get the media result from the paginated nodes
    pub fn media(&self) -> Option<&Vec<AnilistMedia>> {
        match self {
            AnilistPaginatedNodes::Media { media } => Some(media),
            _ => None,
        }
    }
}

/// The result of an airing schedule search
#[derive(Debug, Clone)]
pub struct AnilistAiringSchedulePaged {
    /// The airing schedules result
    pub airing_schedules: Vec<AnilistAiringSchedule>,
    /// The page information
    pub page_info: AnilistPageInfo,
}

impl Default for AnilistAiringSchedulePaged {
    fn default() -> Self {
        AnilistAiringSchedulePaged {
            airing_schedules: Vec::new(),
            page_info: AnilistPageInfo {
                total: 0,
                per_page: 50,
                current_page: 1,
                has_next_page: false,
            },
        }
    }
}

/// The inner data of the Anilist page result
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AnilistPageInnerData {
    /// The nodes of the page, can be either airing schedules or media
    #[serde(flatten)]
    pub nodes: AnilistPaginatedNodes,
    /// The page information
    #[serde(rename = "pageInfo")]
    pub page_info: AnilistPageInfo,
}

/// The inner data of the Anilist page result
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AnilistPagedData {
    /// The inner data of the page
    #[serde(rename = "Page")]
    pub page: AnilistPageInnerData,
}

/// The response of a single media query from the Anilist API
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AnilistSingleMedia {
    /// The media result of the query
    #[serde(rename = "Media")]
    pub media: AnilistMedia,
}

/// The response from the Anilist API
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AnilistResponse<T> {
    /// The data inside the response
    #[serde(bound(deserialize = "T: Deserialize<'de>", serialize = "T: Serialize"))]
    pub data: T,
}

/// Error response schema for GraphQL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnilistGraphQLResponseError {
    /// The collection of errors
    pub errors: Vec<AnilistGraphQLError>,
}

/// An error from the GraphQL response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnilistGraphQLError {
    /// The error message
    pub message: String,
    /// The locations of the error in the query
    pub locations: Vec<AnilistGraphQLErrorLocation>,
    /// The path of the error in the query
    pub path: Vec<serde_json::Value>,
}

/// The location of an error in the GraphQL response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnilistGraphQLErrorLocation {
    /// The line of the error
    pub line: u32,
    /// The column of the error
    pub column: u32,
}

impl std::fmt::Display for AnilistGraphQLError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Error: {msg} (at {line}, {column}[..] in {path})
        write!(f, "Error: {}", self.message)?;
        if !self.locations.is_empty() {
            write!(
                f,
                " (at {})",
                self.locations
                    .iter()
                    .map(|loc| format!("{}:{}", loc.line, loc.column))
                    .collect::<Vec<String>>()
                    .join(", ")
            )?;
        }
        if !self.path.is_empty() {
            // stringify path and write it
            let path_str = self
                .path
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
                .join(".");
            write!(f, " in {path_str}")?;
        }
        // newline
        writeln!(f)
    }
}

impl std::fmt::Display for AnilistGraphQLResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.errors.is_empty() {
            writeln!(f, "No errors found")?;
        } else if self.errors.len() == 1 {
            writeln!(f, "{}", self.errors[0])?;
        } else {
            // print with numbering
            for (i, error) in self.errors.iter().enumerate() {
                writeln!(f, "[{}] {}", i + 1, error)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for AnilistGraphQLError {}
impl std::error::Error for AnilistGraphQLResponseError {}

/// Error type for Anilist API
///
/// This enum can be used to wrap the possible errors that can happen when
/// interacting with the Anilist API.
#[derive(Debug)]
pub enum AnilistError {
    /// Error related to GraphQL request
    GraphQL(AnilistGraphQLResponseError),
    /// Error related to request
    Request(reqwest::Error),
    /// Error related to deserialization
    Serde(Box<DetailedSerdeError>),
    /// Conversion to string failure from header
    HeaderToString(String),
    /// String conversion failure from header
    StringToNumber(String),
}

impl From<DetailedSerdeError> for AnilistError {
    fn from(value: DetailedSerdeError) -> Self {
        AnilistError::Serde(Box::new(value))
    }
}

impl From<reqwest::Error> for AnilistError {
    fn from(value: reqwest::Error) -> Self {
        AnilistError::Request(value)
    }
}

impl From<AnilistGraphQLResponseError> for AnilistError {
    fn from(value: AnilistGraphQLResponseError) -> Self {
        AnilistError::GraphQL(value)
    }
}

impl std::fmt::Display for AnilistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnilistError::GraphQL(e) => write!(f, "{e}"),
            AnilistError::Request(e) => write!(f, "{e}"),
            AnilistError::Serde(e) => write!(f, "{e}"),
            AnilistError::HeaderToString(c) => {
                write!(f, "failed to convert `{c}` header value into string")
            }
            AnilistError::StringToNumber(c) => {
                write!(f, "failed to convert `{c}` header value into number")
            }
        }
    }
}

impl std::error::Error for AnilistError {}

#[cfg(test)]
mod tests {
    #[test]
    fn serialize_media_type() {
        use super::AnilistMediaType;

        let anime = AnilistMediaType::Anime;
        let manga = AnilistMediaType::Manga;

        assert_eq!(serde_json::to_string(&anime).unwrap(), r#""ANIME""#);
        assert_eq!(serde_json::to_string(&manga).unwrap(), r#""MANGA""#);
    }

    #[test]
    fn serialize_media_format() {
        use super::AnilistMediaFormat;

        let tv = AnilistMediaFormat::Tv;
        let tv_short = AnilistMediaFormat::TvShort;
        let movie = AnilistMediaFormat::Movie;
        let special = AnilistMediaFormat::Special;
        let ova = AnilistMediaFormat::OVA;
        let ona = AnilistMediaFormat::ONA;
        let music = AnilistMediaFormat::Music;
        let manga = AnilistMediaFormat::Manga;
        let novel = AnilistMediaFormat::Novel;
        let one_shot = AnilistMediaFormat::OneShot;

        assert_eq!(serde_json::to_string(&tv).unwrap(), r#""TV""#);
        assert_eq!(serde_json::to_string(&tv_short).unwrap(), r#""TV_SHORT""#);
        assert_eq!(serde_json::to_string(&movie).unwrap(), r#""MOVIE""#);
        assert_eq!(serde_json::to_string(&special).unwrap(), r#""SPECIAL""#);
        assert_eq!(serde_json::to_string(&ova).unwrap(), r#""OVA""#);
        assert_eq!(serde_json::to_string(&ona).unwrap(), r#""ONA""#);
        assert_eq!(serde_json::to_string(&music).unwrap(), r#""MUSIC""#);
        assert_eq!(serde_json::to_string(&manga).unwrap(), r#""MANGA""#);
        assert_eq!(serde_json::to_string(&novel).unwrap(), r#""NOVEL""#);
        assert_eq!(serde_json::to_string(&one_shot).unwrap(), r#""ONE_SHOT""#);
    }

    #[test]
    fn test_deser_on_paginated() {
        let test_str = r#"{
            "data": {
                "Page": {
                    "airingSchedules": [
                        {
                            "id": 374988,
                            "episode": 5,
                            "airingAt": 1696600800,
                            "mediaId": 154587
                        },
                        {
                            "id": 375011,
                            "episode": 28,
                            "airingAt": 1711116000,
                            "mediaId": 154587
                        }
                    ],
                    "pageInfo": {
                        "total": 24,
                        "perPage": 50,
                        "currentPage": 1,
                        "lastPage": 1,
                        "hasNextPage": false
                    }
                }
            }
        }"#;

        let data: super::AnilistResponse<super::AnilistPagedData> =
            serde_json::from_str(test_str).unwrap();

        assert_eq!(
            data.data.page.nodes,
            super::AnilistPaginatedNodes::AiringSchedules {
                airing_schedules: vec![
                    super::AnilistAiringSchedule {
                        id: 374988,
                        episode: 5,
                        airing_at: 1696600800,
                    },
                    super::AnilistAiringSchedule {
                        id: 375011,
                        episode: 28,
                        airing_at: 1711116000,
                    }
                ]
            }
        );
    }
}
