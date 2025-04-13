use super::{WebhookEngine, WebhookEnginePayload};

pub struct DiscordEngine {
    pub(crate) url: String,
    pub(crate) name: String,
    pub(crate) avatar: Option<String>,
    pub(crate) locale: showtimes_i18n::Language,
}

impl DiscordEngine {
    /// Create a new Discord engine
    pub fn new(url: impl Into<String>, name: impl Into<String>, avatar: Option<String>) -> Self {
        Self {
            url: url.into(),
            name: name.into(),
            avatar,
            locale: showtimes_i18n::Language::default(),
        }
    }
}

impl WebhookEngine for DiscordEngine {
    fn method(&self) -> reqwest::Method {
        reqwest::Method::POST
    }

    fn set_locale(&mut self, locale: showtimes_i18n::Language) {
        self.locale = locale;
    }
}

impl WebhookEnginePayload for DiscordEngine {
    fn project_create(
        &self,
        project: &showtimes_db::m::Project,
    ) -> Result<reqwest::Body, super::WebhookEnginePayloadError> {
        let description = showtimes_i18n::tr(
            "project-create-desc",
            Some(self.locale),
            &[
                ("kind", project.kind.to_locale().to_string()),
                ("name", format!("**{}**", project.title)),
            ],
        );

        let payload = serde_json::json!({
            "username": self.name,
            "avatar_url": self.avatar,
            "embeds": [
                {
                    "title": showtimes_i18n::t("project-create", Some(self.locale)),
                    "description": description,
                    "color": 0x33FFAD,
                }
            ]
        });

        Ok(reqwest::Body::from(serde_json::to_string(&payload)?))
    }

    fn project_dropped(
        &self,
        project: &showtimes_db::m::Project,
    ) -> Result<reqwest::Body, super::WebhookEnginePayloadError> {
        let description = showtimes_i18n::tr(
            "project-dropped-desc",
            Some(self.locale),
            &[("name", format!("**{}**", project.title))],
        );

        let payload = serde_json::json!({
            "username": self.name,
            "avatar_url": self.avatar,
            "embeds": [
                {
                    "title": showtimes_i18n::t("project-dropped", Some(self.locale)),
                    "description": description,
                    "color": 0xFF3333,
                }
            ]
        });

        Ok(reqwest::Body::from(serde_json::to_string(&payload)?))
    }

    fn project_resumed(
        &self,
        project: &showtimes_db::m::Project,
    ) -> Result<reqwest::Body, super::WebhookEnginePayloadError> {
        let description = showtimes_i18n::tr(
            "project-resumed-desc",
            Some(self.locale),
            &[("name", format!("**{}**", project.title))],
        );

        let payload = serde_json::json!({
            "username": self.name,
            "avatar_url": self.avatar,
            "embeds": [
                {
                    "title": showtimes_i18n::t("project-resumed", Some(self.locale)),
                    "description": description,
                    "color": 0x33FFAD,
                }
            ]
        });

        Ok(reqwest::Body::from(serde_json::to_string(&payload)?))
    }

    fn project_release(
        &self,
        project: &showtimes_db::m::Project,
        episode: &showtimes_db::m::EpisodeProgress,
    ) -> Result<reqwest::Body, super::WebhookEnginePayloadError> {
        // TODO: Make adjustment

        // TODO: Full payload
        let payload = serde_json::json!({
            "username": self.name,
            "avatar_url": self.avatar,
        });

        Ok(reqwest::Body::from(serde_json::to_string(&payload)?))
    }

    fn project_release_multi(
        &self,
        project: &showtimes_db::m::Project,
        episodes: &[showtimes_db::m::EpisodeProgress],
    ) -> Result<reqwest::Body, super::WebhookEnginePayloadError> {
        // TODO: Make adjustment

        // TODO: Full payload
        let payload = serde_json::json!({
            "username": self.name,
            "avatar_url": self.avatar,
        });

        Ok(reqwest::Body::from(serde_json::to_string(&payload)?))
    }

    fn project_unrelease(
        &self,
        project: &showtimes_db::m::Project,
        episode: &showtimes_db::m::EpisodeProgress,
    ) -> Result<reqwest::Body, super::WebhookEnginePayloadError> {
        // TODO: Make adjustment

        // TODO: Full payload
        let payload = serde_json::json!({
            "username": self.name,
            "avatar_url": self.avatar,
        });

        Ok(reqwest::Body::from(serde_json::to_string(&payload)?))
    }

    fn project_unrelease_multi(
        &self,
        project: &showtimes_db::m::Project,
        episodes: &[showtimes_db::m::EpisodeProgress],
    ) -> Result<reqwest::Body, super::WebhookEnginePayloadError> {
        // TODO: Make adjustment

        // TODO: Full payload
        let payload = serde_json::json!({
            "username": self.name,
            "avatar_url": self.avatar,
        });

        Ok(reqwest::Body::from(serde_json::to_string(&payload)?))
    }

    fn project_progress(
        &self,
        project: &showtimes_db::m::Project,
        before: &showtimes_db::m::EpisodeProgress,
        after: &showtimes_db::m::EpisodeProgress,
    ) -> Result<reqwest::Body, super::WebhookEnginePayloadError> {
        // TODO: Make adjustment

        // TODO: Full payload
        let payload = serde_json::json!({
            "username": self.name,
            "avatar_url": self.avatar,
        });

        Ok(reqwest::Body::from(serde_json::to_string(&payload)?))
    }

    fn project_progress_batch(
        &self,
        project: &showtimes_db::m::Project,
        pairs: &[(
            showtimes_db::m::EpisodeProgress,
            showtimes_db::m::EpisodeProgress,
        )],
    ) -> Result<reqwest::Body, super::WebhookEnginePayloadError> {
        // TODO: Make adjustment

        // TODO: Full payload
        let payload = serde_json::json!({
            "username": self.name,
            "avatar_url": self.avatar,
        });

        Ok(reqwest::Body::from(serde_json::to_string(&payload)?))
    }
}
