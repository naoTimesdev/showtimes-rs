use super::{IntegrationId, ShowModelHandler};
use bson::serde_helpers::chrono_datetime_as_bson_datetime;
use serde::{Deserialize, Serialize};
use showtimes_shared::ulid_serializer;

const DEFAULT_MESSAGE_DISPLAY: &str = ":newspaper::mega: | Rilisan Baru: **{title}**\n{link}";

/// A structure to hold the display information for a RSS feed.
///
/// This structure follows Discord rich embeds formatting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RSSFeedEmbedDisplay {
    /// The title of the RSS feed.
    pub title: Option<String>,
    /// The description of the RSS feed.
    pub description: Option<String>,
    /// The URL of the RSS feed.
    pub url: Option<String>,
    /// The thumbnail URL of the RSS feed.
    pub thumbnail: Option<String>,
    /// The image URL of the RSS feed.
    pub image: Option<String>,
    /// The footer of the RSS feed.
    pub footer: Option<String>,
    /// The footer image icon URL of the RSS feed.
    pub footer_image: Option<String>,
    /// The author of the RSS feed.
    ///
    /// Default to naoTimes Feed
    pub author: Option<String>,
    /// The author icon URL of the RSS feed.
    ///
    /// Default to naoTimes logo
    pub author_image: Option<String>,
    /// The color of the RSS feed. This is an optional field, and is not
    /// required.
    pub color: Option<u32>,
    /// A boolean indicating whether the RSS feed is timestamped or not. This
    /// is an optional field, and is not required.
    pub timestamped: bool,
}

impl RSSFeedEmbedDisplay {
    pub fn displayable(&self) -> bool {
        // Check if any important field is set
        self.title.is_some()
            || self.description.is_some()
            || self.thumbnail.is_some()
            || self.image.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RSSFeedDisplay {
    /// The default message for the RSS feed.
    pub message: Option<String>,
    /// The embed display for the RSS feed.
    pub embed: Option<RSSFeedEmbedDisplay>,
}

impl RSSFeedDisplay {
    pub fn new(message: impl Into<String>) -> Self {
        RSSFeedDisplay {
            message: Some(message.into()),
            embed: None,
        }
    }

    pub fn new_with_embed(message: impl Into<String>, embed: RSSFeedEmbedDisplay) -> Self {
        RSSFeedDisplay {
            message: Some(message.into()),
            embed: Some(embed),
        }
    }
}

impl Default for RSSFeedDisplay {
    fn default() -> Self {
        Self {
            message: Some(DEFAULT_MESSAGE_DISPLAY.to_string()),
            embed: None,
        }
    }
}

/// A model to hold RSS information for a server.
#[derive(Debug, Clone, Serialize, Deserialize, showtimes_derive::ShowModelHandler)]
#[col_name("ShowtimesRSSFeed")]
pub struct RSSFeed {
    /// The RSS feed ID
    #[serde(with = "ulid_serializer")]
    pub id: showtimes_shared::ulid::Ulid,
    /// RSS feed URL
    pub url: url::Url,
    /// The RSS integrations, usually Discord text channels.
    pub integrations: Vec<IntegrationId>,
    /// Is this feed enabled or not
    pub enabled: bool,
    /// The display information for the RSS feed
    pub display: RSSFeedDisplay,
    /// Last modified date
    pub last_mod: Option<String>,
    /// Last E-Tag of the RSS feed
    pub etag: Option<String>,
    /// The feed creator (server ID)
    #[serde(with = "ulid_serializer")]
    pub creator: showtimes_shared::ulid::Ulid,
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<mongodb::bson::oid::ObjectId>,
    #[serde(
        with = "chrono_datetime_as_bson_datetime",
        default = "chrono::Utc::now"
    )]
    pub created: chrono::DateTime<chrono::Utc>,
    #[serde(
        with = "chrono_datetime_as_bson_datetime",
        default = "chrono::Utc::now"
    )]
    pub updated: chrono::DateTime<chrono::Utc>,
}
