use crate::{ShowModelHandler, impl_trait_model};
use serde::{Deserialize, Serialize};

/// A model to hold migrations information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    /// The migration's ID described
    pub name: String,
    #[serde(
        with = "showtimes_shared::bson_datetime_jiff_timestamp",
        default = "jiff::Timestamp::now"
    )]
    pub ts: jiff::Timestamp,
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<mongodb::bson::oid::ObjectId>,
    #[serde(
        with = "showtimes_shared::bson_datetime_jiff_timestamp",
        default = "jiff::Timestamp::now"
    )]
    pub updated: jiff::Timestamp,
    pub is_current: bool,
}

impl_trait_model!(Migration, "ShowtimesMigrations", _id, updated);

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
