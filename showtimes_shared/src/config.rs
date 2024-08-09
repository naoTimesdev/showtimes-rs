//! Shared config handler for Showtimes

use serde::Deserialize;

/// Database connection configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Database {
    /// The URL of the MongoDB server
    pub mongodb: String,
}

/// Meilisearch configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Meilisearch {
    /// The URL of the Meilisearch server
    pub url: String,
    /// The master/API key for the Meilisearch server
    pub api_key: String,
}

/// Discord OAuth2 configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DiscordOAuth2 {
    /// The client ID of the Discord OAuth2 application
    pub client_id: String,
    /// The client secret of the Discord OAuth2 application
    pub client_secret: String,
    /// The redirect URL for the Discord OAuth2 application
    pub redirect_url: String,
}

/// The external or metadata services API key
#[derive(Debug, Clone, Deserialize)]
pub struct ExternalServices {
    /// The TMDb API key
    pub tmdb: Option<String>,
    /// The VNDB API key
    pub vndb: Option<String>,
}

/// Storage configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Storages {
    pub local: Option<StorageLocal>,
    pub s3: Option<StorageS3>,
}

impl Storages {
    /// Check if any storage is available
    pub fn is_available(&self) -> bool {
        self.local.is_some() || self.s3.is_some()
    }
}

/// Local storage configuration
#[derive(Debug, Clone, Deserialize)]
pub struct StorageLocal {
    /// The path for the local storage
    pub path: String,
}

/// S3 storage configuration
#[derive(Debug, Clone, Deserialize)]
pub struct StorageS3 {
    /// The bucket name for the S3 storage
    pub bucket: String,
    /// The region for the S3 storage
    pub region: String,
    /// The access key for the S3 storage
    pub access_key: String,
    /// The secret key for the S3 storage
    pub secret_key: String,
    /// The endpoint URL override for the S3 storage
    pub endpoint_url: Option<String>,
}

/// The full configuration for Showtimes
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// The host to bind the server
    ///
    /// Default to `None` which will bind to `localhost`
    pub host: Option<String>,
    /// The port to bind the server
    ///
    /// Default to `5560`
    pub port: Option<u16>,
    /// The database connection configuration
    pub database: Database,
    /// The Meilisearch configuration
    #[serde(rename = "search")]
    pub meilisearch: Meilisearch,
    /// The Discord OAuth2 configuration
    pub discord: DiscordOAuth2,
    /// The external or metadata services API key
    pub external: ExternalServices,
    /// The storage configuration
    pub storages: Storages,
}

impl Config {
    fn with_defaults(&mut self) {
        if self.host.is_none() {
            self.host = Some("127.0.0.1".to_string());
        }

        if self.port.is_none() {
            self.port = Some(5560);
        }
    }

    /// Load the configuration from the file path
    pub fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let buffer = std::fs::read_to_string(path)?;

        let mut config: Self = toml::from_str(&buffer)?;
        config.with_defaults();

        Ok(config)
    }

    /// Load the configuration from the file path in async context
    pub async fn async_load(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let file = tokio::fs::read(path).await?;
        let buffer = String::from_utf8(file.to_vec())?;

        let mut config: Self = toml::from_str(&buffer)?;
        config.with_defaults();

        Ok(config)
    }

    pub fn verify(&self) -> anyhow::Result<()> {
        if !self.storages.is_available() {
            return Err(anyhow::anyhow!("No storage is configured"));
        }

        Ok(())
    }
}
