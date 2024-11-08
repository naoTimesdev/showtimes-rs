use chrono::TimeZone;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnilistMediaType {
    Anime,
    Manga,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnilistMediaFormat {
    Tv,
    TvShort,
    Movie,
    Special,
    #[serde(rename = "OVA")]
    OVA,
    #[serde(rename = "ONA")]
    ONA,
    Music,
    Manga,
    Novel,
    OneShot,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
pub struct AnilistFuzzyDate {
    pub year: Option<i32>,
    pub month: Option<i32>,
    pub day: Option<i32>,
}

impl AnilistFuzzyDate {
    /// Parse the fuzzy date into a [`chrono::DateTime`]
    pub fn into_chrono(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        match (self.year, self.month, self.day) {
            (Some(year), Some(month), Some(day)) => chrono::Utc
                .with_ymd_and_hms(year, month as u32, day as u32, 0, 0, 0)
                .single(),
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
            self.month.map_or(String::new(), |m| format!("-{:02}", m)),
            self.day.map_or(String::new(), |d| format!("-{:02}", d)),
        )
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
pub struct AnilistAiringSchedule {
    pub id: i32,
    pub episode: i32,
    #[serde(rename = "airingAt")]
    pub airing_at: i64,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct AnilistCoverImage {
    pub medium: Option<String>,
    pub large: Option<String>,
    #[serde(rename = "extraLarge")]
    pub extra_large: Option<String>,
    pub color: Option<String>,
}

impl AnilistCoverImage {
    pub fn get_image(&self) -> Option<String> {
        self.extra_large
            .clone()
            .or_else(|| self.large.clone())
            .or_else(|| self.medium.clone())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct AnilistMediaTitle {
    pub romaji: Option<String>,
    pub english: Option<String>,
    pub native: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct AnilistMediaScheduleResult {
    pub airing_schedule: AnilistAiringSchedule,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct AnilistMedia {
    pub id: i32,
    #[serde(rename = "idMal")]
    pub id_mal: Option<i32>,
    pub title: AnilistMediaTitle,
    #[serde(rename = "coverImage")]
    pub cover_image: AnilistCoverImage,
    #[serde(rename = "startDate")]
    pub start_date: Option<AnilistFuzzyDate>,
    pub season: Option<String>,
    #[serde(rename = "seasonYear")]
    pub season_year: Option<i32>,
    pub episodes: Option<i32>,
    pub chapters: Option<i32>,
    pub volumes: Option<i32>,
    pub format: AnilistMediaFormat,
    #[serde(rename = "type")]
    pub kind: AnilistMediaType,
    pub description: Option<String>,
    #[serde(rename = "isAdult")]
    pub is_adult: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnilistPageInfo {
    pub total: i32,
    pub per_page: i32,
    pub current_page: i32,
    pub has_next_page: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum AnilistPaginatedNodes {
    AiringSchedules {
        #[serde(rename = "airingSchedules")]
        airing_schedules: Vec<AnilistAiringSchedule>,
    },
    Media {
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

#[derive(Debug, Clone)]
pub struct AnilistAiringSchedulePaged {
    pub airing_schedules: Vec<AnilistAiringSchedule>,
    pub page_info: AnilistPageInfo,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AnilistPageInnerData {
    #[serde(flatten)]
    pub nodes: AnilistPaginatedNodes,
    #[serde(rename = "pageInfo")]
    pub page_info: AnilistPageInfo,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AnilistPagedData {
    #[serde(rename = "Page")]
    pub page: AnilistPageInnerData,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AnilistSingleMedia {
    #[serde(rename = "Media")]
    pub media: AnilistMedia,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AnilistResponse<T> {
    #[serde(bound(deserialize = "T: Deserialize<'de>", serialize = "T: Serialize"))]
    pub data: T,
}

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
