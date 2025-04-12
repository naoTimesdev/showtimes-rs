use crate::ShowModelHandler;
use serde::{Deserialize, Serialize};

/// A model to hold migrations information
#[derive(Debug, Clone, Serialize, Deserialize, showtimes_derive::ShowModelHandler)]
#[col_name("ShowtimesMigrations")]
pub struct Migration {
    /// The migration's ID described
    pub name: String,
    #[serde(
        with = "jiff::fmt::serde::timestamp::second::required",
        default = "jiff::Timestamp::now"
    )]
    pub ts: jiff::Timestamp,
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<mongodb::bson::oid::ObjectId>,
    #[serde(
        with = "jiff::fmt::serde::timestamp::second::required",
        default = "jiff::Timestamp::now"
    )]
    pub updated: jiff::Timestamp,
    pub is_current: bool,
}

impl Migration {
    pub fn new(name: &str, ts: jiff::Timestamp) -> Self {
        Self {
            name: name.to_string(),
            ts,
            _id: None,
            updated: jiff::Timestamp::now(),
            is_current: true,
        }
    }
}
