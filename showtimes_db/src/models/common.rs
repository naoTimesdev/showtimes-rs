use serde::{Deserialize, Serialize};

/// A metadata struct for an image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    /// The type of the image
    pub kind: String,
    /// The key of the image (usually the project ID)
    pub key: String,
    /// The filename of the image
    pub filename: String,
    /// The format of the image
    pub format: String,
    /// The parent of the image (usually the server ID)
    pub parent: Option<String>,
}

impl ImageMetadata {
    pub fn new(
        kind: impl Into<String>,
        key: impl Into<String>,
        filename: impl Into<String>,
        format: impl Into<String>,
        parent: Option<impl Into<String>>,
    ) -> Self {
        ImageMetadata {
            kind: kind.into(),
            key: key.into(),
            filename: filename.into(),
            format: format.into(),
            parent: parent.map(|p| p.into()),
        }
    }

    pub fn stub_with_name(name: impl Into<String>) -> Self {
        ImageMetadata {
            kind: "images".to_string(),
            key: "stubbed".to_string(),
            filename: name.into(),
            format: "png".to_string(),
            parent: None,
        }
    }

    /// Create a URL or path to the image
    pub fn as_url(&self) -> String {
        match &self.parent {
            Some(parent) => format!(
                "/{}/{}/{}/{}",
                &self.kind, parent, &self.key, &self.filename
            ),
            None => format!("/{}/{}/{}", &self.kind, &self.key, &self.filename),
        }
    }
}

impl Default for ImageMetadata {
    fn default() -> Self {
        ImageMetadata {
            kind: "images".to_string(),
            key: "default".to_string(),
            filename: "default.png".to_string(),
            format: "png".to_string(),
            parent: None,
        }
    }
}

/// The list of possible integration types.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, showtimes_derive::SerdeAutomata, showtimes_derive::EnumName,
)]
#[serde_automata(serialize_rename_all = "SCREAMING_SNAKE_CASE", case_sensitive = false)]
pub enum IntegrationType {
    // Related to Discord
    /// A Discord Role ID
    #[serde_automata(deser_rename = "discordrole, discord_role")]
    DiscordRole,
    /// A Discord User ID
    #[serde_automata(deser_rename = "discorduser, discord_user")]
    DiscordUser,
    /// A Discord Text Channel ID
    #[serde_automata(
        ser_rename = "DISCORD_TEXT_CHANNEL",
        deser_rename = "discordchannel, discord_channel, discord_text_channel"
    )]
    DiscordChannel,
    /// A Discord Guild ID
    #[serde_automata(deser_rename = "discordguild, discord_guild")]
    DiscordGuild,
    // Related to FansubDB
    /// Your group FansubDB ID
    #[serde_automata(ser_rename = "FANSUBDB_ID", deser_rename = "fansubdb, fansubdb_id")]
    FansubDB,
    /// A FansubDB Project ID
    #[serde_automata(
        ser_rename = "FANSUBDB_PROJECT_ID",
        deser_rename = "fansubdbproject, fansubdb_project, fansubdb_project_id"
    )]
    FansubDBProject,
    /// A FansubDB Shows ID
    #[serde_automata(
        ser_rename = "FANSUBDB_SHOWS_ID",
        deser_rename = "fansubdbshows, fansubdb_shows, fansubdb_shows_id"
    )]
    FansubDBShows,
    // Related to Providers
    /// Anilist ID
    #[serde_automata(
        ser_rename = "PVD_ANILIST",
        deser_rename = "provideranilist, pvd_anilist, anilist"
    )]
    ProviderAnilist,
    /// Anilist MAL ID mapping
    #[serde_automata(
        ser_rename = "PVD_ANILIST_MAL",
        deser_rename = "provideranilistmal, pvd_anilistmal, pvd_anilist_mal, anilistmal, anilist_mal"
    )]
    ProviderAnilistMal,
    /// VNDB ID
    #[serde_automata(ser_rename = "PVD_VNDB", deser_rename = "providervndb, pvd_vndb, vndb")]
    ProviderVndb,
    /// TMDB ID
    #[serde_automata(ser_rename = "PVD_TMDB", deser_rename = "providertmdb, pvd_tmdb, tmdb")]
    ProviderTmdb,
}

impl IntegrationType {
    /// Check if the integration type is related to Discord
    pub fn is_discord(&self) -> bool {
        matches!(
            self,
            IntegrationType::DiscordRole
                | IntegrationType::DiscordUser
                | IntegrationType::DiscordChannel
                | IntegrationType::DiscordGuild
        )
    }

    /// Check if the integrations type is related to Providers
    pub fn is_provider(&self) -> bool {
        // AnilistMal is not a Provider but just a shortcut from Anilist
        matches!(
            self,
            IntegrationType::ProviderAnilist
                | IntegrationType::ProviderVndb
                | IntegrationType::ProviderTmdb
        )
    }
}

/// Model to hold the ID of an integration.
///
/// This can be used to denote Discord Integration IDs, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationId {
    id: String,
    kind: IntegrationType,
}

impl IntegrationId {
    /// Create a new integration ID
    pub fn new(id: impl Into<String>, kind: IntegrationType) -> Self {
        IntegrationId {
            id: id.into(),
            kind,
        }
    }

    /// Getter for the ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Getter for the kind
    pub fn kind(&self) -> &IntegrationType {
        &self.kind
    }

    /// Set the ID
    pub fn set_id(&mut self, id: impl Into<String>) {
        self.id = id.into();
    }

    /// Set the kind
    pub fn set_kind(&mut self, kind: IntegrationType) {
        self.kind = kind;
    }
}

impl PartialEq for IntegrationId {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.kind == other.kind
    }
}
