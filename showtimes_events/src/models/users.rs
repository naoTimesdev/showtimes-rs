use serde::{Deserialize, Serialize};

/// A user created event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCreatedEvent {
    #[serde(with = "showtimes_shared::ulid_serializer")]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserUpdatedDataEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key: Option<showtimes_shared::APIKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<showtimes_db::m::UserKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar: Option<showtimes_db::m::ImageMetadata>,
}

impl UserUpdatedDataEvent {
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = Some(name.into());
    }

    pub fn set_api_key(&mut self, api_key: showtimes_shared::APIKey) {
        self.api_key = Some(api_key);
    }

    pub fn set_kind(&mut self, kind: showtimes_db::m::UserKind) {
        self.kind = Some(kind);
    }

    pub fn set_avatar(&mut self, avatar: &showtimes_db::m::ImageMetadata) {
        self.avatar = Some(avatar.clone());
    }
}

/// A user updated event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUpdatedEvent {
    #[serde(with = "showtimes_shared::ulid_serializer")]
    id: showtimes_shared::ulid::Ulid,
    before: UserUpdatedDataEvent,
    after: UserUpdatedDataEvent,
}

impl UserUpdatedEvent {
    pub fn new(
        id: showtimes_shared::ulid::Ulid,
        before: UserUpdatedDataEvent,
        after: UserUpdatedDataEvent,
    ) -> Self {
        Self { id, before, after }
    }
}
