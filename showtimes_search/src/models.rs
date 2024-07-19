use serde::{Deserialize, Serialize};
use showtimes_derive::SearchModel;
use showtimes_shared::ulid_serializer;

#[derive(Debug, Clone, Serialize, Deserialize, Default, SearchModel)]
#[search(
    name = "nt:projects",
    filterable = ["id", "parent"],
    searchable = ["id", "title", "aliases", "parent"],
)]
pub struct Project {
    /// The unique identifier of the project
    #[serde(with = "ulid_serializer", default = "ulid_serializer::default")]
    #[primary_key]
    pub id: showtimes_shared::ulid::Ulid,
    /// The title of the project
    pub title: String,
    /// The poster URL of the project
    pub poster_url: Option<String>,
    /// The integrations of the project
    pub integrations: Vec<showtimes_db::models::IntegrationId>,
    /// The aliases of the project
    pub aliases: Vec<String>,
    /// The parent server or creator
    #[serde(with = "ulid_serializer", default = "ulid_serializer::default")]
    pub parent: showtimes_shared::ulid::Ulid,
}

impl From<showtimes_db::m::Project> for Project {
    fn from(value: showtimes_db::m::Project) -> Self {
        let poster_url = value.poster.image.as_url();
        Self {
            id: value.id,
            title: value.title,
            poster_url: Some(poster_url),
            integrations: value.integrations,
            aliases: value.aliases,
            parent: value.creator,
        }
    }
}
