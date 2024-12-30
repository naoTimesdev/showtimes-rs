#![warn(missing_docs, clippy::empty_docs, rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../README.md")]

pub mod manager;
pub mod markdown;
pub mod template;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use feed_rs::model::{MediaContent, MediaThumbnail};
use markdown::expand_url;
use serde::{Deserialize, Serialize};
pub use template::{format_text, VecString};

fn create_client() -> Result<reqwest::Client, reqwest::Error> {
    let ua = format!(
        "showtimes-rss-rs/{} (+https://github.com/naoTimesdev/showtimes-rs)",
        env!("CARGO_PKG_VERSION")
    );

    reqwest::ClientBuilder::new()
        .user_agent(ua)
        .http2_adaptive_window(true)
        .use_rustls_tls()
        .build()
}

fn create_parser(url: &str) -> feed_rs::parser::Parser {
    feed_rs::parser::Builder::new()
        .base_uri(Some(url))
        .sanitize_content(true)
        .build()
}

/// Tests if the given feed is valid.
pub async fn test_feed_validity(feed_url: impl AsRef<str>) -> Result<bool, RSSError> {
    let url = feed_url.as_ref();
    let parsed_url = reqwest::Url::parse(url)?;
    let client = create_client()?;

    let data = client.get(parsed_url.clone()).send().await?;

    if data.status().is_success() {
        let text = data.text().await?;

        let parser = create_parser(parsed_url.as_str());
        match parser.parse(text.as_bytes()) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    } else {
        Ok(false)
    }
}

/// The feed value information
#[derive(Clone, Debug)]
pub enum FeedValue {
    /// Value is a string
    String(String),
    /// Value ia a collection of String
    Collection(VecString),
    /// Value is a timestamp
    Timestamp(DateTime<Utc>),
}

impl From<String> for FeedValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<VecString> for FeedValue {
    fn from(value: VecString) -> Self {
        Self::Collection(value)
    }
}

impl From<Vec<String>> for FeedValue {
    fn from(value: Vec<String>) -> Self {
        Self::Collection(VecString::from(value))
    }
}

impl From<DateTime<Utc>> for FeedValue {
    fn from(value: DateTime<Utc>) -> Self {
        Self::Timestamp(value)
    }
}

impl Serialize for FeedValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            FeedValue::String(s) => serializer.serialize_str(s),
            FeedValue::Collection(s) => s.serialize(serializer),
            FeedValue::Timestamp(s) => s.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for FeedValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct FeedValueVisitor;

        impl<'de> serde::de::Visitor<'de> for FeedValueVisitor {
            type Value = FeedValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or a sequence of strings")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                // Try RFC3339 first
                if let Ok(dt) = value.parse::<DateTime<Utc>>() {
                    Ok(FeedValue::from(dt))
                } else {
                    // Fallback to string
                    Ok(FeedValue::from(value.to_string()))
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut strings = Vec::new();
                while let Some(s) = seq.next_element::<String>()? {
                    strings.push(s);
                }
                Ok(FeedValue::from(strings))
            }
        }

        deserializer.deserialize_any(FeedValueVisitor)
    }
}

impl std::fmt::Display for FeedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeedValue::String(s) => write!(f, "{}", s),
            FeedValue::Collection(s) => write!(f, "{}", s),
            FeedValue::Timestamp(s) => {
                // Use similar format to RFC3339
                write!(f, "{}", s.format("%a, %d %b %Y %H:%M:%S %Z"))
            }
        }
    }
}

/// A map of feed entries
///
/// The key is the name of the field and the value is the field value
pub type FeedEntry<'a> = HashMap<&'a str, FeedValue>;

/// A map of feed entries
pub type FeedEntryCloned = HashMap<String, FeedValue>;

/// A vector of feed entries
pub type FeedEntries<'a> = Vec<FeedEntry<'a>>;

/// A parsed feed information
///
/// This struct contains the title of the feed, the feed entries, the URL of the feed,
/// the etag of the feed, and the last modified date of the feed.
pub struct FeedParsed<'a> {
    /// The title of the feed
    pub title: Option<String>,
    /// A vector of feed entries
    pub entries: FeedEntries<'a>,
    /// The URL of the feed
    pub url: url::Url,
    /// The etag of the feed, if any
    pub etag: Option<String>,
    /// The last modified date of the feed, if any
    pub last_modified: Option<String>,
}

/// Request the given feed and return the parsed feed
pub async fn parse_feed<'a>(
    feed_url: impl AsRef<str>,
    headers: Option<reqwest::header::HeaderMap>,
) -> Result<FeedParsed<'a>, RSSError> {
    let url = feed_url.as_ref();
    let parsed_url = reqwest::Url::parse(url)?;
    let client = create_client()?;

    let data = client
        .get(parsed_url.clone())
        .headers(headers.unwrap_or_default())
        .send()
        .await?
        .error_for_status()?;

    let etags = data
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string());
    let last_modified = data
        .headers()
        .get(reqwest::header::LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string());

    let text = data.text().await?;

    let parser = create_parser(parsed_url.as_str());
    let parsed = parser.parse(text.as_bytes())?;

    let real_url = parsed
        .links
        .first()
        .and_then(|link| url::Url::parse(&link.href).ok())
        .unwrap_or(parsed_url.clone());

    let reparsed_values: FeedEntries<'a> = parsed
        .entries
        .iter()
        .filter_map(|entry| {
            let mut hash_entries: FeedEntry<'a> = HashMap::new();
            let base_url = entry
                .base
                .clone()
                .and_then(|b| url::Url::parse(&b).ok())
                .unwrap_or(real_url.clone());

            let mut can_be_added = false;
            if !entry.id.is_empty() {
                can_be_added = true;
                hash_entries.insert("id", entry.id.clone().into());
            }

            if let Some(title) = &entry.title {
                hash_entries.insert("title", title.content.clone().into());
            }

            if let Some(updated) = entry.updated {
                hash_entries.insert("updated", updated.into());
            }

            if let Some(published) = entry.published {
                hash_entries.insert("published", published.into());
            }

            let parsed_links: Vec<String> = entry
                .links
                .iter()
                .filter_map(|link| markdown::expand_url(&link.href, &base_url).ok())
                .collect();

            if let Some(link) = parsed_links.first() {
                can_be_added = true;
                hash_entries.insert("link", link.to_string().into());
            }

            if parsed_links.len() >= 2 {
                hash_entries.insert("links", parsed_links.into());
            }

            if let Some(content) = &entry.content {
                if let Some(content_body) = &content.body {
                    let parsed = markdown::html_to_markdown(content_body, &base_url);

                    match parsed {
                        Ok(parsed) => {
                            hash_entries.insert("content", parsed.into());
                        }
                        Err(_) => {
                            hash_entries.insert("content", content_body.clone().into());
                        }
                    }
                }
            }

            if let Some(summary) = &entry.summary {
                let content_body = &summary.content;
                if !content_body.is_empty() {
                    let parsed = markdown::html_to_markdown(content_body, &base_url);

                    match parsed {
                        Ok(parsed) => {
                            hash_entries.insert("summary", parsed.into());
                        }
                        Err(_) => {
                            hash_entries.insert("summary", content_body.clone().into());
                        }
                    }
                }
            }

            let authors: Vec<String> = entry
                .authors
                .iter()
                .filter_map(|author| {
                    if !author.name.is_empty() {
                        Some(author.name.to_string())
                    } else {
                        None
                    }
                })
                .collect();

            if !authors.is_empty() {
                hash_entries.insert("authors", authors.clone().into());
                hash_entries.insert("creators", authors.into());
            }

            let categories: Vec<String> = entry
                .categories
                .iter()
                .filter_map(|category| {
                    if !category.term.is_empty() {
                        Some(category.term.to_string())
                    } else {
                        None
                    }
                })
                .collect();

            if !categories.is_empty() {
                hash_entries.insert("categories", categories.into());
            }

            let contributors: Vec<String> = entry
                .contributors
                .iter()
                .filter_map(|author| {
                    if !author.name.is_empty() {
                        Some(author.name.to_string())
                    } else {
                        None
                    }
                })
                .collect();

            if !contributors.is_empty() {
                hash_entries.insert("contributors", contributors.into());
            }

            if let Some(rights) = &entry.rights {
                if !rights.content.is_empty() {
                    hash_entries.insert("rights", rights.content.clone().into());
                }
            }

            let mut media_content = entry
                .media
                .iter()
                .filter_map(|media| {
                    let contents = media
                        .content
                        .iter()
                        .filter_map(|content| {
                            if content.url.is_some() {
                                Some(content.clone())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<MediaContent>>();

                    if contents.is_empty() {
                        None
                    } else {
                        Some(contents)
                    }
                })
                .flatten()
                .collect::<Vec<MediaContent>>();

            media_content.sort_by(|a, b| b.height.unwrap_or(0).cmp(&a.height.unwrap_or(0)));

            if !media_content.is_empty() {
                let first_url = media_content[0].url.clone().map(|a| a.to_string()).unwrap();

                let expanded = expand_url(&first_url, &base_url).unwrap_or(first_url);
                hash_entries.insert("media_content", expanded.into());
            }

            let mut media_thumbnails = entry
                .media
                .iter()
                .filter_map(|media| {
                    let thumbs = media.thumbnails.clone();

                    if thumbs.is_empty() {
                        None
                    } else {
                        Some(thumbs)
                    }
                })
                .flatten()
                .collect::<Vec<MediaThumbnail>>();

            media_thumbnails.sort_by(|a, b| {
                b.image
                    .height
                    .unwrap_or(0)
                    .cmp(&a.image.height.unwrap_or(0))
            });

            if !media_thumbnails.is_empty() {
                let first_url = media_thumbnails[0].image.uri.to_string();
                let expanded = expand_url(&first_url, &base_url).unwrap_or(first_url);
                hash_entries.insert("media_thumbnail", expanded.into());
            }

            if can_be_added {
                Some(hash_entries)
            } else {
                None
            }
        })
        .collect();

    Ok(FeedParsed {
        title: parsed.title.map(|a| a.content),
        entries: reparsed_values,
        url: parsed_url,
        etag: etags,
        last_modified,
    })
}

/// An error occurred when requesting
pub enum RSSError {
    /// An error occurred when requesting
    Reqwest(reqwest::Error),
    /// An error occurred when deserializing data
    Feed(feed_rs::parser::ParseFeedError),
    /// Failed to parse URL
    InvalidUrl(url::ParseError),
}

impl From<reqwest::Error> for RSSError {
    fn from(e: reqwest::Error) -> Self {
        RSSError::Reqwest(e)
    }
}

impl From<feed_rs::parser::ParseFeedError> for RSSError {
    fn from(e: feed_rs::parser::ParseFeedError) -> Self {
        RSSError::Feed(e)
    }
}

impl From<url::ParseError> for RSSError {
    fn from(value: url::ParseError) -> Self {
        RSSError::InvalidUrl(value)
    }
}

impl std::fmt::Display for RSSError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RSSError::Reqwest(e) => e.fmt(f),
            RSSError::Feed(e) => e.fmt(f),
            RSSError::InvalidUrl(e) => e.fmt(f),
        }
    }
}

impl std::fmt::Debug for RSSError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RSSError::Reqwest(e) => e.fmt(f),
            RSSError::Feed(e) => e.fmt(f),
            RSSError::InvalidUrl(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for RSSError {}

/// Converts a `FeedEntry` into a `FeedEntryCloned`, which is a clone of its key and value.
/// This is useful if you want to store the feed entry in a struct and need to clone it.
pub fn transform_to_cloned_feed(feed: &FeedEntry<'_>) -> FeedEntryCloned {
    feed.iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}
