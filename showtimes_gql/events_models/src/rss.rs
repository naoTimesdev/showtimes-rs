//! A custom RSS event model that also renders the RSS feed information

use std::collections::BTreeMap;

use async_graphql::{Object, SimpleObject};
use showtimes_db::m::RSSFeedDisplay;
use showtimes_gql_common::{
    data_loader::RSSFeedLoader, errors::GQLError, DataLoader, DateTimeGQL, GQLErrorCode,
    GQLErrorExt, IntegrationIdGQL, UlidGQL,
};
use showtimes_rss::FeedEntryCloned;

/// A rendered text data for an RSS entry
#[derive(SimpleObject)]
#[graphql(name = "RSSEventFormatValueGQL")]
pub struct RSSEventFormatValueGQL {
    /// Markdown formatted text, this is the default format
    markdown: String,
    /// HTML formatted text
    html: String,
    /// Plain text formatted text
    plain: String,
}

/// A rendered text data for an RSS entry
///
/// This structure follows Discord rich embeds formatting.
#[derive(SimpleObject)]
#[graphql(name = "RSSEventEmbedFormatValueGQL")]
pub struct RSSEventEmbedFormatValueGQL {
    /// Title of the embed
    title: Option<RSSEventFormatValueGQL>,
    /// Description of the embed
    description: Option<RSSEventFormatValueGQL>,
    /// URL of the embed
    url: Option<RSSEventFormatValueGQL>,
    /// Thumbnail of the embed
    thumbnail: Option<RSSEventFormatValueGQL>,
    /// Image of the embed
    image: Option<RSSEventFormatValueGQL>,
    /// Footer of the embed
    footer: Option<RSSEventFormatValueGQL>,
    /// Footer image of the embed
    footer_image: Option<RSSEventFormatValueGQL>,
    /// Author of the embed
    author: RSSEventFormatValueGQL,
    /// Author image of the embed
    author_image: RSSEventFormatValueGQL,
    /// Color of the embed
    color: Option<u32>,
    /// A boolean indicating whether the RSS feed is timestamped or not.
    timestamped: bool,
}

/// A rendered entries of the RSS event without the integrations
#[derive(SimpleObject)]
#[graphql(name = "RSSFeedRenderedGQL")]
pub struct RSSFeedRenderedGQL {
    /// The base message of the render text
    message: RSSEventFormatValueGQL,
    /// The embed message of the render text
    embed: Option<RSSEventEmbedFormatValueGQL>,
}

/// A rendered entries of the RSS event
#[derive(SimpleObject)]
#[graphql(name = "RSSEventRenderedGQL")]
pub struct RSSEventRenderedGQL {
    /// The base message of the render text
    message: RSSEventFormatValueGQL,
    /// The embed message of the render text
    embed: Option<RSSEventEmbedFormatValueGQL>,
    /// The integrations of the RSS feed
    integrations: Vec<IntegrationIdGQL>,
}

/// A RSS event
pub struct RSSEventGQL {
    id: UlidGQL,
    feed_id: UlidGQL,
    server_id: UlidGQL,
    hash: String,
    entry: FeedEntryCloned,
    timestamp: DateTimeGQL,
}

#[Object(name = "RSSEventGQL")]
impl RSSEventGQL {
    /// The ID of the event
    async fn id(&self) -> UlidGQL {
        self.id
    }

    /// The feed ID of the event
    async fn feed_id(&self) -> UlidGQL {
        self.feed_id
    }

    /// The server ID of the event
    async fn server_id(&self) -> UlidGQL {
        self.server_id
    }

    /// The hash/link/ID of the RSS entry
    async fn hash(&self) -> String {
        self.hash.clone()
    }

    /// The timestamp of the entry or the event if it is not timestamped
    async fn timestamp(&self) -> DateTimeGQL {
        self.timestamp
    }

    /// The rendered message of the event with integrations information
    async fn rendered(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<RSSEventRenderedGQL> {
        let loader = ctx.data_unchecked::<DataLoader<RSSFeedLoader>>();

        let item = loader.load_one(*self.feed_id).await?.ok_or_else(|| {
            GQLError::new("RSS feed not found", GQLErrorCode::RSSFeedNotFound)
                .extend(|e| e.set("id", self.feed_id.to_string()))
        })?;

        RSSEventRenderedGQL::render(item, self.entry.clone())
    }
}

impl RSSEventRenderedGQL {
    /// Render the RSS feed with the given entry
    pub fn render(
        feed: showtimes_db::m::RSSFeed,
        entries: FeedEntryCloned,
    ) -> async_graphql::Result<Self> {
        let integrations = feed
            .integrations
            .iter()
            .map(IntegrationIdGQL::from)
            .collect();

        let rendered_data = render_feed_display_with_entry(&feed.display, entries)?;

        Ok(Self {
            message: rendered_data.message,
            embed: rendered_data.embed,
            integrations,
        })
    }
}

impl From<showtimes_events::m::RSSEvent> for RSSEventGQL {
    fn from(value: showtimes_events::m::RSSEvent) -> Self {
        let chrono_ts =
            chrono::DateTime::<chrono::Utc>::from_timestamp(value.timestamp().unix_timestamp(), 0)
                .unwrap_or_else(|| {
                    // current time
                    chrono::Utc::now()
                });

        Self {
            id: value.id().into(),
            feed_id: value.feed_id().into(),
            server_id: value.server_id().into(),
            hash: value.hash_key().to_string(),
            entry: value.entry().clone(),
            timestamp: chrono_ts.into(),
        }
    }
}

impl From<&showtimes_events::m::RSSEvent> for RSSEventGQL {
    fn from(value: &showtimes_events::m::RSSEvent) -> Self {
        let chrono_ts =
            chrono::DateTime::<chrono::Utc>::from_timestamp(value.timestamp().unix_timestamp(), 0)
                .unwrap_or_else(|| {
                    // current time
                    chrono::Utc::now()
                });

        Self {
            id: value.id().into(),
            feed_id: value.feed_id().into(),
            server_id: value.server_id().into(),
            hash: value.hash_key().to_string(),
            entry: value.entry().clone(),
            timestamp: chrono_ts.into(),
        }
    }
}

fn render_single(
    when: &str,
    data: &str,
    entries: &BTreeMap<String, showtimes_rss::FeedValue>,
) -> async_graphql::Result<RSSEventFormatValueGQL> {
    let args = vec![];
    let markdown = showtimes_rss::format_text(data, &args, entries).extend_error(
        GQLErrorCode::RSSFeedRenderError,
        |f| {
            f.set("template", data);
            f.set("on", when);
        },
    )?;

    let plain = showtimes_rss::markdown::markdown_to_text(&markdown);
    let html = showtimes_rss::markdown::markdown_to_html(&markdown);

    Ok(RSSEventFormatValueGQL {
        markdown,
        plain,
        html,
    })
}

fn render_single_opt(
    when: &str,
    data: Option<&str>,
    entries: &BTreeMap<String, showtimes_rss::FeedValue>,
) -> async_graphql::Result<Option<RSSEventFormatValueGQL>> {
    if let Some(data) = data {
        Ok(Some(render_single(when, data, entries)?))
    } else {
        Ok(None)
    }
}

/// Render a rendered RSS feed display, given the display configuration and the feed entry.
pub fn render_feed_display_with_entry(
    display: &RSSFeedDisplay,
    entries: FeedEntryCloned,
) -> async_graphql::Result<RSSFeedRenderedGQL> {
    let raw_msg = display
        .message
        .clone()
        .unwrap_or_else(|| RSSFeedDisplay::default_message().to_string());

    let entries: BTreeMap<_, _> = entries
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();

    let embed_display = if let Some(embed) = &display.embed {
        let title = render_single_opt("embed.title", embed.title.as_deref(), &entries)?;
        let description =
            render_single_opt("embed.description", embed.description.as_deref(), &entries)?;
        let url = render_single_opt("embed.url", embed.url.as_deref(), &entries)?;
        let thumbnail = render_single_opt("embed.thumbnail", embed.thumbnail.as_deref(), &entries)?;
        let image = render_single_opt("embed.image", embed.image.as_deref(), &entries)?;
        let footer = render_single_opt("embed.footer", embed.footer.as_deref(), &entries)?;
        let footer_image = render_single_opt(
            "embed.footer_image",
            embed.footer_image.as_deref(),
            &entries,
        )?;
        let author = render_single(
            "embed.author",
            embed.author.as_deref().unwrap_or("naoTimes Feed"),
            &entries,
        )?;
        let author_image = render_single(
            "embed.author_image",
            embed
                .author_image
                .as_deref()
                .unwrap_or("https://naoti.me/assets/img/nt256.png"),
            &entries,
        )?;

        Some(RSSEventEmbedFormatValueGQL {
            title,
            description,
            url,
            thumbnail,
            image,
            footer,
            footer_image,
            author,
            author_image,
            color: embed.color,
            timestamped: embed.timestamped,
        })
    } else {
        None
    };

    Ok(RSSFeedRenderedGQL {
        message: render_single("message", &raw_msg, &entries)?,
        embed: embed_display,
    })
}
