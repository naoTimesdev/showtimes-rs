pub(crate) mod discord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AvailableEngine {
    Discord,
}

/// Super trait for all webhook engines
pub trait WebhookEngine {
    /// The request method for the webhook
    fn method(&self) -> reqwest::Method;
    /// Store the prefered locale
    ///
    /// This will force each engine to use the same locale
    fn set_locale(&mut self, locale: showtimes_i18n::Language);
}

/// The list of payload generator for an engine
///
/// Each engine need to implement this trait
///
/// This needs to be synced with [`showtimes_db::m::WebhookAction`]
pub trait WebhookEnginePayload: WebhookEngine {
    /// The project creation payload
    fn project_create(
        &self,
        project: &showtimes_db::m::Project,
    ) -> Result<reqwest::Body, WebhookEnginePayloadError>;
    /// The project progress payload
    fn project_progress(
        &self,
        project: &showtimes_db::m::Project,
        before: &showtimes_db::m::EpisodeProgress,
        after: &showtimes_db::m::EpisodeProgress,
    ) -> Result<reqwest::Body, WebhookEnginePayloadError>;
    /// The project progress payload
    fn project_progress_batch(
        &self,
        project: &showtimes_db::m::Project,
        pairs: &[(
            showtimes_db::m::EpisodeProgress,
            showtimes_db::m::EpisodeProgress,
        )],
    ) -> Result<reqwest::Body, WebhookEnginePayloadError>;
    /// The project release payload
    fn project_release(
        &self,
        project: &showtimes_db::m::Project,
        episode: &showtimes_db::m::EpisodeProgress,
    ) -> Result<reqwest::Body, WebhookEnginePayloadError>;
    /// The project release multi payload
    fn project_release_multi(
        &self,
        project: &showtimes_db::m::Project,
        episodes: &[showtimes_db::m::EpisodeProgress],
    ) -> Result<reqwest::Body, WebhookEnginePayloadError>;
    /// The project un-release payload
    fn project_unrelease(
        &self,
        project: &showtimes_db::m::Project,
        episode: &showtimes_db::m::EpisodeProgress,
    ) -> Result<reqwest::Body, WebhookEnginePayloadError>;
    /// The project un-release multi payload
    fn project_unrelease_multi(
        &self,
        project: &showtimes_db::m::Project,
        episodes: &[showtimes_db::m::EpisodeProgress],
    ) -> Result<reqwest::Body, WebhookEnginePayloadError>;
    /// The project dropped payload
    fn project_dropped(
        &self,
        project: &showtimes_db::m::Project,
    ) -> Result<reqwest::Body, WebhookEnginePayloadError>;
    /// The project resumed payload
    fn project_resumed(
        &self,
        project: &showtimes_db::m::Project,
    ) -> Result<reqwest::Body, WebhookEnginePayloadError>;
}

/// An error that can happen when generating a payload
#[derive(Debug)]
pub enum WebhookEnginePayloadError {
    /// The payload is invalid
    InvalidPayload(serde_json::Error),
}

impl std::fmt::Display for WebhookEnginePayloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebhookEnginePayloadError::InvalidPayload(err) => write!(f, "Invalid payload: {}", err),
        }
    }
}

impl From<serde_json::Error> for WebhookEnginePayloadError {
    fn from(err: serde_json::Error) -> Self {
        WebhookEnginePayloadError::InvalidPayload(err)
    }
}
