use crate::ShowModelHandler;
use bson::serde_helpers::chrono_datetime_as_bson_datetime;
use serde::{Deserialize, Serialize};

/// A model to hold migrations information
#[derive(Debug, Clone, Serialize, Deserialize, showtimes_derive::ShowModelHandler)]
#[col_name("ShowtimesMigrations")]
pub struct Migration {
    /// The migration's ID described
    pub name: String,
    #[serde(
        with = "chrono_datetime_as_bson_datetime",
        default = "chrono::Utc::now"
    )]
    pub ts: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<mongodb::bson::oid::ObjectId>,
    #[serde(
        with = "chrono_datetime_as_bson_datetime",
        default = "chrono::Utc::now"
    )]
    pub updated: chrono::DateTime<chrono::Utc>,
    pub is_current: bool,
}

impl Migration {
    pub fn new(name: &str, ts: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            name: name.to_string(),
            ts,
            _id: None,
            updated: chrono::Utc::now(),
            is_current: true,
        }
    }
}
