use serde::{Deserialize, Serialize};

use super::projects::Project;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCollabConfirm {
    pub id: String,
    pub server_id: String,
    pub anime_id: String,
}

/// Mapped into `showtimesdatas` collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub id: String,
    pub name: Option<String>,
    pub fsdb_id: Option<u32>,
    pub serverowner: String,
    pub announce_channel: Option<String>,
    pub anime: Vec<Project>,
    pub konfirmasi: Vec<ServerCollabConfirm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<mongodb::bson::oid::ObjectId>,
}

/// Mapped into `showtimesadmin` collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerAdmin {
    pub admin_id: String,
    #[serde(default)]
    pub servers: Vec<String>,
}
