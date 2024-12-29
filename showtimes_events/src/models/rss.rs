//! A collection of RSS events model
//!
//! This is custom made and not related to the other events model.

use super::{deserialize_ulid, serialize_ulid};
use clickhouse::Row;
use serde::{Deserialize, Serialize};
use showtimes_rss::{transform_to_cloned_feed, FeedEntry, FeedEntryCloned, FeedValue};
use std::fmt::Debug;

/// The event structure that is broadcasted and stored
#[derive(Clone, Debug, Row, Serialize, Deserialize)]
pub struct RSSEvent {
    /// The ID of the event, this is randomly generated
    #[serde(
        deserialize_with = "deserialize_ulid",
        serialize_with = "serialize_ulid"
    )]
    id: showtimes_shared::ulid::Ulid,
    /// The Feed ID associated with this event
    #[serde(
        deserialize_with = "deserialize_ulid",
        serialize_with = "serialize_ulid"
    )]
    feed_id: showtimes_shared::ulid::Ulid,
    /// The Server ID associated with this event
    #[serde(
        deserialize_with = "deserialize_ulid",
        serialize_with = "serialize_ulid"
    )]
    server_id: showtimes_shared::ulid::Ulid,
    /// The event data itself, on Clickhouse this will be stored as a string
    entries: FeedEntryCloned,
    /// The timestamp of the event
    #[serde(with = "clickhouse::serde::time::datetime")]
    timestamp: ::time::OffsetDateTime,
}

impl RSSEvent {
    /// Create a new `RSSEvent` from a single [`FeedEntry`]
    ///
    /// The `published_at` field of the [`FeedEntry`] is used to determine the
    /// timestamp of the event. If the field is not found, the current time is used.
    /// The `FeedEntry` is cloned and the [`FeedEntry`]s are converted to [`FeedEntryCloned`].
    pub fn from_entry(
        feed: showtimes_shared::ulid::Ulid,
        server: showtimes_shared::ulid::Ulid,
        entry: &FeedEntry,
    ) -> Self {
        let default_time = ::time::OffsetDateTime::now_utc();
        let published_at = if let Some(FeedValue::Timestamp(published_at)) =
            entry.get("published").or_else(|| entry.get("updated"))
        {
            ::time::OffsetDateTime::from_unix_timestamp(published_at.timestamp())
                .unwrap_or(default_time)
        } else {
            default_time
        };

        let clone_entry = transform_to_cloned_feed(entry);

        Self {
            id: showtimes_shared::ulid_serializer::default(),
            feed_id: feed,
            server_id: server,
            entries: clone_entry,
            timestamp: published_at,
        }
    }

    /// Get the ID of the event
    pub fn id(&self) -> showtimes_shared::ulid::Ulid {
        self.id
    }

    /// Get the feed ID of the event
    pub fn feed_id(&self) -> showtimes_shared::ulid::Ulid {
        self.feed_id
    }

    /// Get the server ID of the event
    pub fn server_id(&self) -> showtimes_shared::ulid::Ulid {
        self.server_id
    }

    /// Get the data/entry of the event
    pub fn entry(&self) -> &FeedEntryCloned {
        &self.entries
    }

    /// Get the timestamp of the event
    pub fn timestamp(&self) -> ::time::OffsetDateTime {
        self.timestamp
    }
}
