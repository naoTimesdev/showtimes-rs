//! The manager for RSS data, powered via Redis/Valkey

use std::collections::HashMap;
use std::sync::Arc;

use redis::cmd;
use redis::AsyncCommands;
use redis::RedisResult;

use crate::FeedEntry;
use crate::FeedEntryCloned;
use crate::FeedValue;

/// The shared [`RSSManager`] instance for the showtimes service.
///
/// Can be used between threads safely.
pub type SharedRSSManager = std::sync::Arc<tokio::sync::Mutex<RSSManager>>;
const RSS_MANAGER_BASE: &str = "showtimes:rss";

/// Redis-managed RSS state for the showtimes service.
#[derive(Debug, Clone)]
pub struct RSSManager {
    connection: redis::aio::MultiplexedConnection,
}

impl RSSManager {
    /// Create a new RSS manager.
    pub async fn new(client: &Arc<redis::Client>) -> RedisResult<Self> {
        let client_name = format!("showtimes-rs/{}", env!("CARGO_PKG_VERSION"));

        let mut con = client.get_multiplexed_async_connection().await?;
        // Test the connection
        cmd("PING").exec_async(&mut con).await?;

        // Set the client name
        cmd("CLIENT")
            .arg("SETNAME")
            .arg(client_name)
            .exec_async(&mut con)
            .await?;

        Ok(Self { connection: con })
    }

    /// Push a new entry
    pub async fn push_entry<'a>(
        &mut self,
        feed: impl Into<String>,
        entry: &FeedEntry<'a>,
    ) -> RedisResult<()> {
        let entry_key = make_entry_key(&entry);
        let rss_key = format!("{}:{}", RSS_MANAGER_BASE, feed.into());

        self.connection.sadd(rss_key, entry_key).await
    }

    /// Push multiple new entries
    pub async fn push_entries<'a>(
        &mut self,
        feed: impl Into<String>,
        entries: &[FeedEntry<'a>],
    ) -> RedisResult<()> {
        let entry_key: Vec<String> = entries.iter().map(make_entry_key).collect();
        let rss_key = format!("{}:{}", RSS_MANAGER_BASE, feed.into());

        self.connection.sadd(rss_key, entry_key).await
    }

    /// Flush all entries
    pub async fn flush_entries(&mut self, feed: impl Into<String>) -> RedisResult<()> {
        let rss_key = format!("{}:{}", RSS_MANAGER_BASE, feed.into());

        self.connection.del(rss_key).await
    }

    /// Check if the following keys exist or not.
    pub async fn keys_exist<'a>(
        &mut self,
        feed: impl Into<String>,
        entries: &[FeedEntry<'a>],
    ) -> RedisResult<HashMap<String, bool>> {
        let rss_key = format!("{}:{}", RSS_MANAGER_BASE, feed.into());

        let entries_keys = entries.iter().map(make_entry_key).collect::<Vec<String>>();

        let results: Vec<bool> = self
            .connection
            .smismember(rss_key, entries_keys.clone())
            .await?;

        let mapped_results: HashMap<String, bool> = entries_keys
            .iter()
            .zip(results)
            .map(|(k, v)| (k.to_string(), v))
            .collect();

        Ok(mapped_results)
    }

    /// Check if the following keys exist or not.
    ///
    /// This version is for the [`FeedEntryCloned`] type which is from ClickHouse.
    pub async fn keys_exist_cloned(
        &mut self,
        feed: impl Into<String>,
        entries: &[FeedEntryCloned],
    ) -> RedisResult<HashMap<String, bool>> {
        let rss_key = format!("{}:{}", RSS_MANAGER_BASE, feed.into());

        let entries_keys = entries
            .iter()
            .map(make_entry_key_cloned)
            .collect::<Vec<String>>();

        let results: Vec<bool> = self
            .connection
            .smismember(rss_key, entries_keys.clone())
            .await?;

        let mapped_results: HashMap<String, bool> = entries_keys
            .iter()
            .zip(results)
            .map(|(k, v)| (k.to_string(), v))
            .collect();

        Ok(mapped_results)
    }
}

/// Create a new entry key from a [`FeedEntry`].
///
/// This will be either `id` or `link`, two of them are guaranteed to exist.
pub fn make_entry_key<'a>(feed: &FeedEntry<'a>) -> String {
    // This is a guarantee!
    let id = feed.get("id");
    let url = feed.get("link");

    match (id, url) {
        (Some(FeedValue::String(id)), _) => id.to_string(),
        (_, Some(FeedValue::String(url))) => url.to_string(),
        _ => unreachable!("Should never happen"),
    }
}

/// Create a new entry key from a [`FeedEntryCloned`].
///
/// Similar to [`make_entry_key`], this is a guarantee.
pub fn make_entry_key_cloned(feed: &FeedEntryCloned) -> String {
    // This is a guarantee!
    let id = feed.get("id");
    let url = feed.get("link");

    match (id, url) {
        (Some(FeedValue::String(id)), _) => id.to_string(),
        (_, Some(FeedValue::String(url))) => url.to_string(),
        _ => unreachable!("Should never happen"),
    }
}
