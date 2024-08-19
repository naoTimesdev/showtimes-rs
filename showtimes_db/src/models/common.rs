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
#[derive(Debug, Copy, Clone, tosho_macros::DeserializeEnum, tosho_macros::SerializeEnum)]
pub enum IntegrationType {
    // Related to Discord
    /// A Discord Role ID
    DiscordRole,
    /// A Discord User ID
    DiscordUser,
    /// A Discord Text Channel ID
    DiscordChannel,
    /// A Discord Guild ID
    DiscordGuild,
    // Related to FansubDB
    /// Your group FansubDB ID
    FansubDB,
    /// A FansubDB Project ID
    FansubDBProject,
    /// A FansubDB Shows ID
    FansubDBShows,
    // Related to Providers
    /// Anilist ID
    ProviderAnilist,
    /// Anilist MAL ID mapping
    ProviderAnilistMal,
    /// VNDB ID
    ProviderVndb,
    /// TMDB ID
    ProviderTmdb,
}

tosho_macros::enum_error!(IntegrationTypeFromStrError);

impl std::str::FromStr for IntegrationType {
    type Err = IntegrationTypeFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // lowercase the string
        let s_lower = s.to_lowercase();
        match s_lower.as_str() {
            "discordrole" | "discord_role" => Ok(IntegrationType::DiscordRole),
            "discorduser" | "discord_user" => Ok(IntegrationType::DiscordUser),
            "discordchannel" | "discord_channel" | "discord_text_channel" => {
                Ok(IntegrationType::DiscordChannel)
            }
            "discordguild" | "discord_guild" => Ok(IntegrationType::DiscordGuild),
            "fansubdb" | "fansubdb_id" => Ok(IntegrationType::FansubDB),
            "fansubdbproject" | "fansubdb_project" | "fansubdb_project_id" => {
                Ok(IntegrationType::FansubDBProject)
            }
            "fansubdbshows" | "fansubdb_shows" | "fansubdb_shows_id" => {
                Ok(IntegrationType::FansubDBShows)
            }
            "provideranilist" | "pvd_anilist" | "anilist" => Ok(IntegrationType::ProviderAnilist),
            "provideranilistmal" | "pvd_anilistmal" | "pvd_anilist_mal" | "anilistmal"
            | "anilist_mal" => Ok(IntegrationType::ProviderAnilistMal),
            "providervndb" | "pvd_vndb" | "vndb" => Ok(IntegrationType::ProviderVndb),
            "providertmdb" | "pvd_tmdb" | "tmdb" => Ok(IntegrationType::ProviderTmdb),
            _ => Err(IntegrationTypeFromStrError {
                original: s.to_string(),
            }),
        }
    }
}

impl std::fmt::Display for IntegrationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegrationType::DiscordRole => write!(f, "DISCORD_ROLE"),
            IntegrationType::DiscordUser => write!(f, "DISCORD_USER"),
            IntegrationType::DiscordChannel => write!(f, "DISCORD_TEXT_CHANNEL"),
            IntegrationType::DiscordGuild => write!(f, "DISCORD_GUILD"),
            IntegrationType::FansubDB => write!(f, "FANSUBDB_ID"),
            IntegrationType::FansubDBProject => write!(f, "FANSUBDB_PROJECT_ID"),
            IntegrationType::FansubDBShows => write!(f, "FANSUBDB_SHOWS_ID"),
            IntegrationType::ProviderAnilist => write!(f, "PVD_ANILIST"),
            IntegrationType::ProviderAnilistMal => write!(f, "PVD_ANILIST_MAL"),
            IntegrationType::ProviderVndb => write!(f, "PVD_VNDB"),
            IntegrationType::ProviderTmdb => write!(f, "PVD_TMDB"),
        }
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
