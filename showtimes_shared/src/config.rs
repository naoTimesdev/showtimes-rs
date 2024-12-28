//! Shared config handler for Showtimes

use serde::Deserialize;

const DEFAULT_SECRET: &str = "super-duper-secret-jwt-key";
const DEFAULT_MASTER_KEY: &str = "masterkey";
const EXPIRY_DEFAULT: u64 = 7 * 24 * 60 * 60;

/// JWT session configuration
#[derive(Debug, Clone, Deserialize)]
pub struct JwtSession {
    /// The secret key for the JWT session
    pub secret: String,
    /// The expiration time for the JWT session
    ///
    /// By default, it is set to 7 days.
    /// Set to 0 to disable expiration.
    #[serde(default)]
    pub expiration: Option<u64>,
}

impl JwtSession {
    /// Get the expiration time for a JWT session, or return the default value
    pub fn get_expiration(&self) -> u64 {
        self.expiration.unwrap_or(EXPIRY_DEFAULT)
    }
}

/// Database connection configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Database {
    /// The URL of the MongoDB server
    pub mongodb: String,
    /// The URL of the Redis server
    pub redis: String,
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
    #[serde(default)]
    pub tmdb: Option<String>,
    /// The VNDB API key
    #[serde(default)]
    pub vndb: Option<String>,
}

/// Storage configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Storages {
    /// Local storage configuration
    #[serde(default)]
    pub local: Option<StorageLocal>,
    /// S3 storage configuration
    #[serde(default)]
    pub s3: Option<StorageS3>,
    /// Enable or disable the image proxy feature
    #[serde(default)]
    pub disable_proxy: Option<bool>,
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

/// The path style used for S3 bucket
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum StorageS3PathStyle {
    /// Virtual host path style ({bucket}.s3.{region}.amazonaws.com)
    #[default]
    Virtual,
    /// Path style (s3.{region}.amazonaws.com/{bucket})
    Path,
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
    /// The endpoint URL for the S3 storage
    pub endpoint_url: String,
    /// Path style for the bucket in S3 storage
    #[serde(default)]
    pub path_style: StorageS3PathStyle,
}

/// ClickHouse configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ClickHouseEvent {
    /// The url of the ClickHouse server
    pub url: String,
    /// The username of the ClickHouse server
    pub username: String,
    /// The password of the ClickHouse server
    #[serde(default)]
    pub password: Option<String>,
}

/// The full configuration for Showtimes
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// The host to bind the server
    ///
    /// Default to `None` which will bind to `localhost`
    #[serde(default)]
    pub host: Option<String>,
    /// The port to bind the server
    ///
    /// Default to `5560`
    #[serde(default)]
    pub port: Option<u16>,
    /// The port to bind the metrics/tokio-console server
    ///
    /// Default to `5562`
    #[serde(rename = "tokio-port", default)]
    pub tokio_port: Option<u16>,
    /// The master key for the server
    pub master_key: String,
    /// The log directory for the server
    #[serde(rename = "log-directory", default)]
    pub log_directory: Option<String>,
    /// The database connection configuration
    pub database: Database,
    /// The Meilisearch configuration
    #[serde(rename = "search")]
    pub meilisearch: Meilisearch,
    /// The ClickHouse configuration
    #[serde(rename = "events")]
    pub clickhouse: ClickHouseEvent,
    /// The Discord OAuth2 configuration
    pub discord: DiscordOAuth2,
    /// The external or metadata services API key
    pub external: ExternalServices,
    /// The storage configuration
    pub storages: Storages,
    /// The JWT session configuration
    pub jwt: JwtSession,
}

/// This macro wraps [`ConfigVerifyError`] and the error item &str into a String
macro_rules! bail_verify {
    ($variant:ident, $item:expr) => {{
        return Err(ConfigVerifyError::$variant($item.to_string()).into());
    }};
}

impl Config {
    fn with_defaults(&mut self) {
        if self.host.is_none() {
            self.host = Some("127.0.0.1".to_string());
        }

        if self.port.is_none() {
            self.port = Some(5560);
        }

        if self.tokio_port.is_none() {
            self.tokio_port = Some(5562);
        }

        if self.jwt.expiration.is_none() {
            self.jwt.expiration = Some(EXPIRY_DEFAULT);
        }

        if let Some(log_dir) = &self.log_directory {
            // Check if not empty
            if log_dir.is_empty() {
                self.log_directory = None;
                return;
            }

            // Check if a directory
            let path = std::path::Path::new(log_dir);
            if !path.is_dir() {
                self.log_directory = None;
            }
        }
    }

    /// Load the configuration from the file path
    pub fn load(path: impl AsRef<std::path::Path>) -> ConfigResult<Self> {
        let buffer = std::fs::read_to_string(path)?;

        let mut config: Self = toml::from_str(&buffer)?;
        config.with_defaults();

        Ok(config)
    }

    /// Load the configuration from the file path in async context
    pub async fn async_load(path: impl AsRef<std::path::Path>) -> ConfigResult<Self> {
        let buffer = tokio::fs::read_to_string(path).await?;

        let mut config: Self = toml::from_str(&buffer)?;
        config.with_defaults();

        Ok(config)
    }

    /// Verify provided config if it's fullfill some of the preferred requirements
    pub fn verify(&self) -> ConfigResult<()> {
        // Verify master key
        if self.master_key.is_empty() {
            bail_verify!(Required, "Master key")
        }
        if self.master_key == DEFAULT_MASTER_KEY {
            bail_verify!(NoDefault, "Master key")
        }

        // Verify JWT
        if self.jwt.secret.is_empty() {
            bail_verify!(Required, "JWT secret")
        }
        if self.jwt.secret == DEFAULT_SECRET {
            bail_verify!(NoDefault, "JWT secret")
        }

        // --> Database will be verified when loading the connection
        // --> Meilisearch will be verified when loading the connection
        // --> ClickHouse will be verified when loading the connection

        // Verify Discord OAuth2
        if self.discord.client_id.is_empty() {
            bail_verify!(Required, "Discord OAuth2 client ID")
        }
        if self.discord.client_secret.is_empty() {
            bail_verify!(Required, "Discord OAuth2 client secret")
        }
        if self.discord.redirect_url.is_empty() {
            bail_verify!(Required, "Discord OAuth2 redirect URL")
        }

        if self.discord.client_id == "00000000000000000000" {
            bail_verify!(NoDefault, "Discord OAuth2 client ID")
        }
        if self.discord.client_secret == "supersecretdiscordclientsecret" {
            bail_verify!(NoDefault, "Discord OAuth2 client secret")
        }
        if self.discord.redirect_url.contains("your.naotimes.ui") {
            bail_verify!(NoDefault, "Discord OAuth2 redirect URL")
        }

        // Verify external services
        if let Some(tmdb) = &self.external.tmdb {
            if tmdb.is_empty() {
                bail_verify!(NoDefaultOrNull, "TMDb API key")
            }

            if tmdb == "your-valid-access-token-for-tmdb" {
                bail_verify!(NoDefaultOrNull, "TMDb API key")
            }
        }

        if let Some(vndb) = &self.external.vndb {
            if vndb.is_empty() {
                bail_verify!(NoDefaultOrNull, "VNDB API key")
            }

            if vndb == "your-valid-access-token-for-vndb" {
                bail_verify!(NoDefaultOrNull, "VNDB API key")
            }
        }

        // Verify storage
        if !self.storages.is_available() {
            bail_verify!(Required, "Storage")
        }

        Ok(())
    }
}

/// A wrapper result for config and [`ConfigError`] type.
pub type ConfigResult<T> = Result<T, ConfigError>;

/// A collection of error when loading config or verifying config
#[derive(Debug)]
pub enum ConfigError {
    /// Error when loading config
    LoadError(std::io::Error),
    /// Error when parsing config
    ParseError(toml::de::Error),
    /// Error when verifying config
    VerifyError(ConfigVerifyError),
}

/// A collection of error when verifying config
#[derive(Debug)]
pub enum ConfigVerifyError {
    /// Required field is not set
    Required(String),
    /// Field is not changed from default, we need to change it
    NoDefault(String),
    /// Field is not changed from default, we need to change it or set it to null to disable.
    NoDefaultOrNull(String),
}

impl From<toml::de::Error> for ConfigError {
    fn from(value: toml::de::Error) -> Self {
        ConfigError::ParseError(value)
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        ConfigError::LoadError(value)
    }
}

impl From<ConfigVerifyError> for ConfigError {
    fn from(value: ConfigVerifyError) -> Self {
        ConfigError::VerifyError(value)
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::LoadError(e) => write!(f, "Failed to load config: {}", e),
            ConfigError::ParseError(e) => write!(f, "Failed to parse config: {}", e),
            ConfigError::VerifyError(e) => write!(f, "Failed to verify config: {}", e),
        }
    }
}

impl std::fmt::Display for ConfigVerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigVerifyError::Required(item) => write!(f, "{} is required and not set!", item),
            ConfigVerifyError::NoDefault(item) => {
                write!(f, "{} is not changed from default, please change it!", item)
            }
            ConfigVerifyError::NoDefaultOrNull(item) => write!(
                f,
                "{} is not changed from default, please change it or set to `null` if not used!",
                item
            ),
        }
    }
}

impl std::error::Error for ConfigError {}
impl std::error::Error for ConfigVerifyError {}
