//! A collection of users events model

use serde::{Deserialize, Serialize};
use showtimes_derive::EventModel;

/// A user created event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct UserCreatedEvent {
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
    username: String,
}

impl From<showtimes_db::m::User> for UserCreatedEvent {
    fn from(user: showtimes_db::m::User) -> Self {
        Self {
            id: user.id,
            username: user.username,
        }
    }
}

impl From<&showtimes_db::m::User> for UserCreatedEvent {
    fn from(user: &showtimes_db::m::User) -> Self {
        Self {
            id: user.id,
            username: user.username.clone(),
        }
    }
}

/// A user updated data event
///
/// Used in conjuction with the [`UserUpdatedEvent`]
#[derive(Debug, Clone, Serialize, Deserialize, Default, EventModel)]
pub struct UserUpdatedDataEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[event_copy]
    api_key: Option<showtimes_shared::APIKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[event_copy]
    kind: Option<showtimes_db::m::UserKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar: Option<showtimes_db::m::ImageMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    discord_meta: Option<showtimes_db::m::DiscordUser>,
}

/// A user updated event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct UserUpdatedEvent {
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
    before: UserUpdatedDataEvent,
    after: UserUpdatedDataEvent,
}

impl UserUpdatedEvent {
    /// Create a new user updated event
    pub fn new(
        id: showtimes_shared::ulid::Ulid,
        before: UserUpdatedDataEvent,
        after: UserUpdatedDataEvent,
    ) -> Self {
        Self { id, before, after }
    }
}

/// A user deleted event
#[derive(Debug, Clone, Serialize, Deserialize, EventModel)]
pub struct UserDeletedEvent {
    #[serde(with = "showtimes_shared::ulid_serializer")]
    #[event_copy]
    id: showtimes_shared::ulid::Ulid,
}

impl UserDeletedEvent {
    /// Create a new user deleted event
    pub fn new(id: showtimes_shared::ulid::Ulid) -> Self {
        Self { id }
    }
}

impl From<showtimes_db::m::User> for UserDeletedEvent {
    fn from(user: showtimes_db::m::User) -> Self {
        Self { id: user.id }
    }
}

impl From<&showtimes_db::m::User> for UserDeletedEvent {
    fn from(user: &showtimes_db::m::User) -> Self {
        Self { id: user.id }
    }
}
