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
    pub translator: ProjectAssignee,
    #[serde(default, rename = "TLC")]
    pub translation_checker: ProjectAssignee,
    #[serde(default, rename = "ENC")]
    pub encoder: ProjectAssignee,
    #[serde(default, rename = "ED")]
    pub editor: ProjectAssignee,
    #[serde(default, rename = "TM")]
    pub timer: ProjectAssignee,
    #[serde(default, rename = "TS")]
    pub typesetter: ProjectAssignee,
    #[serde(default, rename = "QC")]
    pub quality_checker: ProjectAssignee,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LastUpdate {
    Flat(i64),
    Comma(f64),
}

impl TryFrom<LastUpdate> for jiff::Timestamp {
    type Error = String;

    fn try_from(value: LastUpdate) -> Result<Self, Self::Error> {
        match value {
            LastUpdate::Flat(v) => {
                jiff::Timestamp::from_second(v).map_err(|_| format!("Invalid timestamp: {}", v))
            }
            LastUpdate::Comma(v) => {
                // Round the float to the nearest second
                let secs = v as i64;
                jiff::Timestamp::from_second(secs).map_err(|_| format!("Invalid timestamp: {}", v))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
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
    pub last_update: LastUpdate,
}
