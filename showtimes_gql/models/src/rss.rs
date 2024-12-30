//! A RSS feed models list

use async_graphql::Object;
use showtimes_db::m::RSSFeedEmbedDisplay;
use showtimes_gql_common::{
    data_loader::ServerDataLoader, errors::GQLError, DataLoader, DateTimeGQL, GQLErrorCode,
    IntegrationIdGQL, UlidGQL,
};

use crate::servers::ServerGQL;

/// The RSS feed object
pub struct RSSFeedGQL(showtimes_db::m::RSSFeed);

#[Object]
impl RSSFeedGQL {
    /// The RSS feed ID
    async fn id(&self) -> UlidGQL {
        self.0.id.into()
    }

    /// The RSS feed URL
    async fn url(&self) -> String {
        self.0.url.to_string()
    }

    /// The integrations of the RSS feed
    async fn integrations(&self) -> Vec<IntegrationIdGQL> {
        self.0
            .integrations
            .iter()
            .map(IntegrationIdGQL::from)
            .collect()
    }

    /// Is the RSS feed enabled?
    async fn enabled(&self) -> bool {
        self.0.enabled
    }

    /// The display information for the RSS feed
    async fn display(&self) -> RSSFeedDisplayGQL {
        RSSFeedDisplayGQL::from(&self.0.display)
    }

    /// The associated server of the RSS feed
    async fn server(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<ServerGQL> {
        let loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();

        let srv = loader.load_one(self.0.creator).await?.ok_or_else(|| {
            GQLError::new("Server not found", GQLErrorCode::ServerNotFound)
                .extend(|e| e.set("id", self.0.creator.to_string()))
        })?;

        let srv_gql: ServerGQL = srv.into();
        Ok(srv_gql.with_projects_disabled())
    }

    /// The RSS feed creation date
    async fn created(&self) -> DateTimeGQL {
        self.0.created.into()
    }

    /// The RSS feed last update date
    async fn updated(&self) -> DateTimeGQL {
        self.0.updated.into()
    }
}

/// The RSS feed display information
pub struct RSSFeedDisplayGQL {
    message: Option<String>,
    embed: Option<RSSFeedEmbedDisplay>,
}

#[Object]
impl RSSFeedDisplayGQL {
    /// The message of the RSS feed
    async fn message(&self) -> Option<String> {
        transform_string(&self.message)
    }

    /// The embed display information of the RSS feed
    async fn embed(&self) -> Option<RSSFeedEmbedDisplayGQL> {
        self.embed.as_ref().map(RSSFeedEmbedDisplayGQL::from)
    }
}

/// The RSS feed embed display information
pub struct RSSFeedEmbedDisplayGQL {
    /// The title of the RSS feed.
    title: Option<String>,
    /// The description of the RSS feed.
    description: Option<String>,
    /// The URL of the RSS feed.
    url: Option<String>,
    /// The thumbnail URL of the RSS feed.
    thumbnail: Option<String>,
    /// The image URL of the RSS feed.
    image: Option<String>,
    /// The footer of the RSS feed.
    footer: Option<String>,
    /// The footer image icon URL of the RSS feed.
    footer_image: Option<String>,
    /// The author of the RSS feed.
    author: Option<String>,
    /// The author icon URL of the RSS feed.
    author_image: Option<String>,
    /// The color of the RSS feed.
    color: Option<u32>,
    /// A boolean indicating whether the RSS feed is timestamped or not.
    timestamped: bool,
}

#[Object]
impl RSSFeedEmbedDisplayGQL {
    /// The title of the RSS feed.
    async fn title(&self) -> Option<String> {
        transform_string(&self.title)
    }

    /// The description of the RSS feed.
    async fn description(&self) -> Option<String> {
        transform_string(&self.description)
    }

    /// The URL of the RSS feed.
    async fn url(&self) -> Option<String> {
        transform_string(&self.url)
    }

    /// The thumbnail URL of the RSS feed.
    async fn thumbnail(&self) -> Option<String> {
        transform_string(&self.thumbnail)
    }

    /// The image URL of the RSS feed.
    async fn image(&self) -> Option<String> {
        transform_string(&self.image)
    }

    /// The footer of the RSS feed.
    async fn footer(&self) -> Option<String> {
        transform_string(&self.footer)
    }

    /// The footer image icon URL of the RSS feed.
    async fn footer_image(&self) -> Option<String> {
        transform_string(&self.footer_image)
    }

    /// The author of the RSS feed.
    async fn author(&self) -> Option<String> {
        transform_string(&self.author)
    }

    /// The author icon URL of the RSS feed.
    async fn author_image(&self) -> Option<String> {
        transform_string(&self.author_image)
    }

    /// The int color of the RSS feed.
    async fn color(&self) -> Option<u32> {
        self.color
    }

    /// A boolean indicating whether the RSS feed is timestamped or not.
    async fn timestamped(&self) -> bool {
        self.timestamped
    }
}

impl From<showtimes_db::m::RSSFeed> for RSSFeedGQL {
    fn from(feed: showtimes_db::m::RSSFeed) -> Self {
        RSSFeedGQL(feed)
    }
}

impl From<&showtimes_db::m::RSSFeed> for RSSFeedGQL {
    fn from(feed: &showtimes_db::m::RSSFeed) -> Self {
        RSSFeedGQL(feed.clone())
    }
}

impl From<showtimes_db::m::RSSFeedEmbedDisplay> for RSSFeedEmbedDisplayGQL {
    fn from(value: showtimes_db::m::RSSFeedEmbedDisplay) -> Self {
        RSSFeedEmbedDisplayGQL {
            title: value.title,
            description: value.description,
            url: value.url,
            thumbnail: value.thumbnail,
            image: value.image,
            footer: value.footer,
            footer_image: value.footer_image,
            author: value.author,
            author_image: value.author_image,
            color: value.color,
            timestamped: value.timestamped,
        }
    }
}

impl From<&showtimes_db::m::RSSFeedEmbedDisplay> for RSSFeedEmbedDisplayGQL {
    fn from(value: &showtimes_db::m::RSSFeedEmbedDisplay) -> Self {
        RSSFeedEmbedDisplayGQL {
            title: value.title.clone(),
            description: value.description.clone(),
            url: value.url.clone(),
            thumbnail: value.thumbnail.clone(),
            image: value.image.clone(),
            footer: value.footer.clone(),
            footer_image: value.footer_image.clone(),
            author: value.author.clone(),
            author_image: value.author_image.clone(),
            color: value.color,
            timestamped: value.timestamped,
        }
    }
}

impl From<showtimes_db::m::RSSFeedDisplay> for RSSFeedDisplayGQL {
    fn from(value: showtimes_db::m::RSSFeedDisplay) -> Self {
        RSSFeedDisplayGQL {
            message: value.message,
            embed: value.embed,
        }
    }
}

impl From<&showtimes_db::m::RSSFeedDisplay> for RSSFeedDisplayGQL {
    fn from(value: &showtimes_db::m::RSSFeedDisplay) -> Self {
        RSSFeedDisplayGQL {
            message: value.message.clone(),
            embed: value.embed.clone(),
        }
    }
}

fn transform_string(s: &Option<String>) -> Option<String> {
    match s {
        Some(s) => {
            if s.trim().is_empty() {
                None
            } else {
                Some(s.clone())
            }
        }
        None => None,
    }
}
