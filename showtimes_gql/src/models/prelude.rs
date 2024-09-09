use std::ops::Deref;

use async_graphql::{
    ComplexObject, Description, Enum, OutputType, Scalar, ScalarType, SimpleObject,
};
use showtimes_db::mongodb::bson::doc;

use super::{projects::ProjectGQL, servers::ServerGQL, users::UserGQL};

/// A wrapper around ULID to allow it to be used in GraphQL
pub struct UlidGQL(showtimes_shared::ulid::Ulid);

impl Deref for UlidGQL {
    type Target = showtimes_shared::ulid::Ulid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Description for UlidGQL {
    fn description() -> &'static str {
        "A ULID (Universally Unique Lexicographically Sortable Identifier) used to uniquely identify objects\nThe following ULID are converted from UUID timestamp or UUIDv7 before being converted to a ULID"
    }
}

#[Scalar(use_type_description = true)]
impl ScalarType for UlidGQL {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        match value {
            async_graphql::Value::String(s) => Ok(UlidGQL(s.parse()?)),
            _ => Err(async_graphql::InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

impl From<showtimes_shared::ulid::Ulid> for UlidGQL {
    fn from(ulid: showtimes_shared::ulid::Ulid) -> Self {
        UlidGQL(ulid)
    }
}

impl From<&showtimes_shared::ulid::Ulid> for UlidGQL {
    fn from(ulid: &showtimes_shared::ulid::Ulid) -> Self {
        UlidGQL(*ulid)
    }
}

/// A wrapper around APIKey to be allowed in GraphQL
pub struct APIKeyGQL(showtimes_shared::APIKey);

impl Deref for APIKeyGQL {
    type Target = showtimes_shared::APIKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Description for APIKeyGQL {
    fn description() -> &'static str {
        "A unique API key used to authenticate users, based on UUIDv4"
    }
}

#[Scalar(use_type_description = true)]
impl ScalarType for APIKeyGQL {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        match value {
            async_graphql::Value::String(s) => {
                let api_key = showtimes_shared::APIKey::from_string(&s)?;

                Ok(APIKeyGQL(api_key))
            }
            _ => Err(async_graphql::InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

impl From<showtimes_shared::APIKey> for APIKeyGQL {
    fn from(ulid: showtimes_shared::APIKey) -> Self {
        APIKeyGQL(ulid)
    }
}

impl From<&showtimes_shared::APIKey> for APIKeyGQL {
    fn from(ulid: &showtimes_shared::APIKey) -> Self {
        APIKeyGQL(*ulid)
    }
}

/// A wrapper around DateTime<Utc> to allow it to be used in GraphQL
#[derive(Clone)]
pub struct DateTimeGQL(
    /// A datetime timestamp format in UTC timezone, follows RFC3339 format
    chrono::DateTime<chrono::Utc>,
);

impl Description for DateTimeGQL {
    fn description() -> &'static str {
        "A datetime timestamp format in UTC timezone, follows RFC3339 format"
    }
}

impl Deref for DateTimeGQL {
    type Target = chrono::DateTime<chrono::Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[Scalar(use_type_description = true)]
impl ScalarType for DateTimeGQL {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        match value {
            async_graphql::Value::String(s) => {
                let rfc3399 = chrono::DateTime::parse_from_rfc3339(&s)?;
                let utc = rfc3399.with_timezone(&chrono::Utc);

                Ok(DateTimeGQL(utc))
            }
            _ => Err(async_graphql::InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_rfc3339())
    }
}

impl From<chrono::DateTime<chrono::Utc>> for DateTimeGQL {
    fn from(dt: chrono::DateTime<chrono::Utc>) -> Self {
        DateTimeGQL(dt)
    }
}

impl From<&chrono::DateTime<chrono::Utc>> for DateTimeGQL {
    fn from(dt: &chrono::DateTime<chrono::Utc>) -> Self {
        DateTimeGQL(*dt)
    }
}

/// Information about an image
#[derive(SimpleObject)]
#[graphql(complex)]
pub struct ImageMetadataGQL {
    /// The type of the image
    kind: String,
    /// The key of the image (usually the project ID)
    key: String,
    /// The filename of the image
    filename: String,
    /// The format of the image
    format: String,
    /// The parent of the image (usually the server ID)
    parent: Option<String>,
}

#[ComplexObject]
impl ImageMetadataGQL {
    /// Get the full URL of the image without the host
    async fn url(&self) -> String {
        match &self.parent {
            Some(parent) => format!(
                "/{}/{}/{}/{}",
                &self.kind, parent, &self.key, &self.filename
            ),
            None => format!("/{}/{}/{}", &self.kind, &self.key, &self.filename),
        }
    }
}

impl From<showtimes_db::m::ImageMetadata> for ImageMetadataGQL {
    fn from(meta: showtimes_db::m::ImageMetadata) -> Self {
        ImageMetadataGQL {
            kind: meta.kind,
            key: meta.key,
            filename: meta.filename,
            format: meta.format,
            parent: meta.parent,
        }
    }
}

impl From<&showtimes_db::m::ImageMetadata> for ImageMetadataGQL {
    fn from(meta: &showtimes_db::m::ImageMetadata) -> Self {
        ImageMetadataGQL {
            kind: meta.kind.clone(),
            key: meta.key.clone(),
            filename: meta.filename.clone(),
            format: meta.format.clone(),
            parent: meta.parent.clone(),
        }
    }
}

/// The list of possible integrations types.
#[derive(Enum, Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[graphql(
    remote = "showtimes_db::m::IntegrationType",
    rename_items = "SCREAMING_SNAKE_CASE"
)]
pub enum IntegrationTypeGQL {
    // Related to Discord
    /// A Discord Role ID
    DiscordRole,
    /// A Discord User ID
    DiscordUser,
    /// A Discord Text Channel ID
    #[graphql(name = "DISCORD_TEXT_CHANNEL")]
    DiscordChannel,
    /// A Discord Guild ID
    DiscordGuild,
    // Related to FansubDB
    /// Your group FansubDB ID
    #[graphql(name = "FANSUBDB_ID")]
    FansubDB,
    /// A FansubDB Project ID
    #[graphql(name = "FANSUBDB_PROJECT_ID")]
    FansubDBProject,
    /// A FansubDB Shows ID
    #[graphql(name = "FANSUBDB_SHOWS_ID")]
    FansubDBShows,
    // Related to Providers
    /// Anilist ID
    #[graphql(name = "PVD_ANILIST")]
    ProviderAnilist,
    /// MyAnimeList ID from Anilist
    #[graphql(name = "PVD_ANILIST_MAL")]
    ProviderAnilistMal,
    /// VNDB ID
    #[graphql(name = "PVD_VNDB")]
    ProviderVndb,
    /// TMDB ID
    #[graphql(name = "PVD_TMDB")]
    ProviderTmdb,
}

impl std::fmt::Display for IntegrationTypeGQL {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegrationTypeGQL::DiscordRole => write!(f, "Discord Role"),
            IntegrationTypeGQL::DiscordUser => write!(f, "Discord User"),
            IntegrationTypeGQL::DiscordChannel => write!(f, "Discord Text Channel"),
            IntegrationTypeGQL::DiscordGuild => write!(f, "Discord Guild"),
            IntegrationTypeGQL::FansubDB => write!(f, "FansubDB ID"),
            IntegrationTypeGQL::FansubDBProject => write!(f, "FansubDB Project ID"),
            IntegrationTypeGQL::FansubDBShows => write!(f, "FansubDB Shows ID"),
            IntegrationTypeGQL::ProviderAnilist => write!(f, "Anilist ID"),
            IntegrationTypeGQL::ProviderAnilistMal => write!(f, "Anilist MAL ID"),
            IntegrationTypeGQL::ProviderVndb => write!(f, "VNDB ID"),
            IntegrationTypeGQL::ProviderTmdb => write!(f, "TMDB ID"),
        }
    }
}

/// A metadata collection to hold integration information with other platform
#[derive(SimpleObject)]
pub struct IntegrationIdGQL {
    /// The ID of the integration
    iod: String,
    /// The kind of the integration
    kind: IntegrationTypeGQL,
}

impl From<showtimes_db::m::IntegrationId> for IntegrationIdGQL {
    fn from(integration: showtimes_db::m::IntegrationId) -> Self {
        IntegrationIdGQL {
            iod: integration.id().to_string(),
            kind: (*integration.kind()).into(),
        }
    }
}

impl From<&showtimes_db::m::IntegrationId> for IntegrationIdGQL {
    fn from(integration: &showtimes_db::m::IntegrationId) -> Self {
        IntegrationIdGQL {
            iod: integration.id().to_string(),
            kind: (*integration.kind()).into(),
        }
    }
}

/// A page information for pagination
#[derive(SimpleObject)]
pub struct PageInfoGQL {
    /// The total number of pages
    total: u64,
    /// The number of items per page
    #[graphql(name = "perPage")]
    per_page: u32,
    /// Next cursor to get the next page
    #[graphql(name = "nextCursor")]
    next_cursor: Option<UlidGQL>,
}

/// A paginated data structure
#[derive(SimpleObject)]
#[graphql(concrete(name = "ProjectPaginatedGQL", params(ProjectGQL)))]
#[graphql(concrete(name = "ServerPaginatedGQL", params(ServerGQL)))]
#[graphql(concrete(name = "UserPaginatedGQL", params(UserGQL)))]
pub struct PaginatedGQL<T: OutputType> {
    /// The items list
    node: Vec<T>,
    /// The page information
    #[graphql(name = "pageInfo")]
    page_info: PageInfoGQL,
}

impl PageInfoGQL {
    /// Create a new PageInfoGQL
    pub fn new(total: u64, per_page: u32, next_cursor: Option<UlidGQL>) -> Self {
        PageInfoGQL {
            total,
            per_page,
            next_cursor,
        }
    }

    /// Empty PageInfoGQL
    pub fn empty(per_page: u32) -> Self {
        PageInfoGQL {
            total: 0,
            per_page,
            next_cursor: None,
        }
    }
}

impl Default for PageInfoGQL {
    fn default() -> Self {
        PageInfoGQL::empty(20)
    }
}

impl<T: OutputType> PaginatedGQL<T> {
    /// Create a new PaginatedGQL
    pub fn new(node: Vec<T>, page_info: PageInfoGQL) -> Self {
        PaginatedGQL { node, page_info }
    }
}

impl<T: OutputType> Default for PaginatedGQL<T> {
    fn default() -> Self {
        PaginatedGQL {
            node: vec![],
            page_info: PageInfoGQL::default(),
        }
    }
}

/// Global sort order for the list
#[derive(Enum, Debug, Default, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum SortOrderGQL {
    /// Sort by ID (Ascending)
    #[default]
    IdAsc,
    /// Sort by ID (Descending)
    IdDesc,
    /// Sort by Name (Ascending)
    NameAsc,
    /// Sort by Name (Descending)
    NameDesc,
    /// Sort by Created At (Ascending)
    CreatedAtAsc,
    /// Sort by Created At (Descending)
    CreatedAtDesc,
    /// Sort by Updated At (Ascending)
    UpdatedAtAsc,
    /// Sort by Updated At (Descending)
    UpdatedAtDesc,
}

impl SortOrderGQL {
    pub(crate) fn into_sort_doc(
        &self,
        title: impl Into<Option<String>>,
    ) -> showtimes_db::mongodb::bson::Document {
        let title: Option<String> = title.into();
        match (self, title) {
            (SortOrderGQL::IdAsc, _) => {
                doc! { "id": 1 }
            }
            (SortOrderGQL::IdDesc, _) => {
                doc! { "id": -1 }
            }
            (SortOrderGQL::NameAsc, Some(title)) => {
                let mut data = showtimes_db::mongodb::bson::Document::new();
                data.insert(title, 1);
                data
            }
            (SortOrderGQL::NameDesc, Some(title)) => {
                let mut data = showtimes_db::mongodb::bson::Document::new();
                data.insert(title, -1);
                data
            }
            (SortOrderGQL::NameAsc, None) => {
                // Fallback to ID
                doc! { "id": 1 }
            }
            (SortOrderGQL::NameDesc, None) => {
                // Fallback to ID
                doc! { "id": -1 }
            }
            (SortOrderGQL::CreatedAtAsc, _) => {
                doc! { "created": 1 }
            }
            (SortOrderGQL::CreatedAtDesc, _) => {
                doc! { "created": -1 }
            }
            (SortOrderGQL::UpdatedAtAsc, _) => {
                doc! { "updated": 1 }
            }
            (SortOrderGQL::UpdatedAtDesc, _) => {
                doc! { "updated": -1 }
            }
        }
    }
}

/// A simple OK response
#[derive(SimpleObject)]
pub struct OkResponse {
    /// The message of the response
    message: String,
    /// The success status of the response
    success: bool,
}

impl OkResponse {
    /// Create a new success OkResponse
    pub fn ok(message: impl Into<String>) -> Self {
        OkResponse {
            message: message.into(),
            success: true,
        }
    }

    /// Create a new error OkResponse
    #[allow(dead_code)]
    pub fn err(message: impl Into<String>) -> Self {
        OkResponse {
            message: message.into(),
            success: false,
        }
    }
}

/// The default roles for each project kind
#[derive(Enum, Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[graphql(
    remote = "showtimes_db::m::ProjectKind",
    rename_items = "SCREAMING_SNAKE_CASE"
)]
pub enum ProjectKindGQL {
    /// The project is a shows, movies, or anything relevant to it
    Shows,
    /// The project is a literature types
    Literature,
    /// The project is a manga types
    Manga,
    /// The project is a games types
    Games,
    /// The project is a unknown type
    Unknown,
}
