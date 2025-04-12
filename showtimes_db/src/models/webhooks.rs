use serde::{Deserialize, Serialize};
use showtimes_shared::ulid_serializer;

use crate::errors::{StringValidationError, StringValidationErrorKind};

use super::ShowModelHandler;

/// The target for the webhook
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WebhookTarget {
    /// Discord webhook target
    Discord,
}

/// Supported webhook actions
///
/// This is all actions that are supported by the webhook
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum WebhookAction {
    /// A new project is created
    ProjectCreate,
    /// A project progress is updated
    ProjectProgress,
    /// A project is released
    ProjectRelease,
    /// A project is reversed/unreleased
    ProjectUnreleased,
    /// A project is dropped
    ProjectDropped,
    /// A project is resumed
    ProjectResumed,
}

impl WebhookAction {
    /// The default actions for the webhook
    pub const DEFAULT_ACTIONS: [WebhookAction; 5] = [
        WebhookAction::ProjectProgress,
        WebhookAction::ProjectRelease,
        WebhookAction::ProjectUnreleased,
        WebhookAction::ProjectDropped,
        WebhookAction::ProjectResumed,
    ];
}

/// The webhook model
#[derive(Debug, Clone, Serialize, Deserialize, showtimes_derive::ShowModelHandler)]
#[col_name("ShowtimesWebhooks")]
pub struct Webhook {
    /// The ID of the webhook
    #[serde(with = "ulid_serializer", default = "ulid_serializer::default")]
    pub id: showtimes_shared::ulid::Ulid,
    /// The target of the webhook
    pub kind: WebhookTarget,
    /// The URL of the webhook
    pub url: String,
    /// The name of the webhook
    ///
    /// This will be used to identify the webhook in the UI and the actual webhook
    ///
    /// Default to `naoTimes` if not set
    #[serde(
        default = "default_webhook_name",
        deserialize_with = "deserialize_name"
    )]
    pub name: String,
    /// The avatar URL of the webhook
    ///
    /// Default to `None` if not set
    #[serde(default)]
    pub avatar: Option<String>,
    /// The supported actions for this Webhook
    pub actions: Vec<WebhookAction>,
    /// The associated server ID for this webhook
    #[serde(with = "ulid_serializer")]
    pub creator: showtimes_shared::ulid::Ulid,
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<mongodb::bson::oid::ObjectId>,
    #[serde(
        with = "jiff::fmt::serde::timestamp::second::required",
        default = "jiff::Timestamp::now"
    )]
    pub created: jiff::Timestamp,
    #[serde(
        with = "jiff::fmt::serde::timestamp::second::required",
        default = "jiff::Timestamp::now"
    )]
    pub updated: jiff::Timestamp,
}

impl Webhook {
    /// Create a new webhook instances
    pub fn new(
        url: impl Into<String>,
        kind: WebhookTarget,
        creator: showtimes_shared::ulid::Ulid,
    ) -> Self {
        let now = jiff::Timestamp::now();
        Self {
            id: showtimes_shared::ulid::Ulid::new(),
            url: url.into(),
            kind,
            name: default_webhook_name(),
            avatar: None,
            actions: WebhookAction::DEFAULT_ACTIONS.to_vec(),
            creator,
            _id: None,
            created: now,
            updated: now,
        }
    }

    /// Set the name of the webhook
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the avatar of the webhook
    pub fn with_avatar(mut self, avatar: impl Into<String>) -> Self {
        self.avatar = Some(avatar.into());
        self
    }

    /// Set the actions of the webhook
    pub fn with_actions(mut self, actions: Vec<WebhookAction>) -> Self {
        self.actions = actions;
        self
    }
}

fn default_webhook_name() -> String {
    "naoTimes".to_string()
}

fn deserialize_name<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // Check if it's either String or null
    let name: Option<String> = Option::deserialize(deserializer)?;
    if let Some(name) = name {
        // Check if it's empty
        if name.is_empty() {
            return Err(serde::de::Error::custom(StringValidationError::new(
                "name",
                StringValidationErrorKind::Empty,
            )));
        }
        Ok(name)
    } else {
        Ok(default_webhook_name())
    }
}
