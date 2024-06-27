use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AnilistFuzzyDate {
    pub year: Option<i32>,
    pub month: Option<i32>,
    pub day: Option<i32>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AnilistAiringScheduleNode {
    pub id: i32,
    pub episode: i32,
    #[serde(rename = "airingAt")]
    pub airing_at: i64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AnilistAiringSchedule {
    pub nodes: Vec<AnilistAiringScheduleNode>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AnilistMediaTitle {
    pub romaji: Option<String>,
    pub english: Option<String>,
    pub native: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AnilistMediaScheduleResult {
    pub airing_schedule: AnilistAiringSchedule,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AnilistAnime {
    pub id: i32,
    pub title: AnilistMediaTitle,
    pub cover_image: AnilistCoverImage,
    pub airing_schedule: AnilistAiringSchedule,
}
