use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeStatusCustomProgress {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub done: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EpisodeStatusProgress {
    #[serde(default, rename = "TL")]
    pub translation: bool,
    #[serde(default, rename = "TLC")]
    pub translation_check: bool,
    #[serde(default, rename = "ENC")]
    pub encoding: bool,
    #[serde(default, rename = "ED")]
    pub editing: bool,
    #[serde(default, rename = "TM")]
    pub timing: bool,
    #[serde(default, rename = "TS")]
    pub typesetting: bool,
    #[serde(default, rename = "QC")]
    pub quality_check: bool,
    #[serde(default)]
    pub custom: Vec<EpisodeStatusCustomProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NumOrFloat {
    Num(u32),
    Float(f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeStatus {
    pub episode: u32,
    pub is_done: bool,
    #[serde(default)]
    pub progress: EpisodeStatusProgress,
    pub airtime: Option<NumOrFloat>,
    pub delay_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectAssignee {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAssigneeCustom {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub person: ProjectAssignee,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectAssignements {
    #[serde(default, rename = "TL")]
    pub translation: ProjectAssignee,
    #[serde(default, rename = "TLC")]
    pub translation_check: ProjectAssignee,
    #[serde(default, rename = "ENC")]
    pub encoding: ProjectAssignee,
    #[serde(default, rename = "ED")]
    pub editing: ProjectAssignee,
    #[serde(default, rename = "TM")]
    pub timing: ProjectAssignee,
    #[serde(default, rename = "TS")]
    pub typesetting: ProjectAssignee,
    #[serde(default, rename = "QC")]
    pub quality_check: ProjectAssignee,
    #[serde(default)]
    pub custom: Vec<ProjectAssigneeCustom>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPoster {
    pub url: String,
    pub color: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFSDB {
    pub id: Option<u32>,
    pub ani_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: u32,
    pub mal_id: Option<u32>,
    pub title: String,
    pub role_id: Option<String>,
    pub start_time: Option<NumOrFloat>,
    #[serde(default)]
    pub assignments: ProjectAssignements,
    pub status: Vec<EpisodeStatus>,
    pub poster_data: ProjectPoster,
    pub fsdb_data: Option<ProjectFSDB>,
    pub aliases: Vec<String>,
    pub kolaborasi: Vec<String>,
    pub last_update: i64,
}
