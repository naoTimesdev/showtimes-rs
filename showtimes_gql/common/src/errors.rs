//! A collection of errors mainly used in the GraphQL schema to return as code.

#[allow(
    clippy::disallowed_types,
    reason = "Allow import since this is used to extend it again later in here."
)]
use async_graphql::{Error, ErrorExtensions};

/// A trait for extending GraphQL errors with additional information.
///
/// This trait provides a method to enhance error reporting in GraphQL responses
/// by adding custom error codes and extension values.
///
/// # Type Parameters
///
/// * `T`: The success type of the result.
/// * `E`: The error type, which must implement [`std::fmt::Display`] and [`std::error::Error`].
pub trait GQLErrorExt<T, E> {
    /// Extends the error with a custom error code and additional extension values.
    ///
    /// # Arguments
    ///
    /// * `code`: The custom [`GQLError`] code to be associated with this error.
    /// * `f`: A closure that modifies the error extension values.
    ///
    /// # Returns
    ///
    /// An [`async_graphql::Result`] with the extended error information.
    ///
    /// # Type Constraints
    ///
    /// * `F`: A closure that takes a mutable reference to [`async_graphql::ErrorExtensionValues`].
    /// * `E`: Must implement [`std::fmt::Display`] and [`std::error::Error`].
    fn extend_error<F>(self, code: GQLErrorCode, f: F) -> async_graphql::Result<T>
    where
        F: FnOnce(&mut async_graphql::ErrorExtensionValues),
        E: std::fmt::Display;
}

impl<T, E> GQLErrorExt<T, E> for Result<T, E>
where
    E: std::fmt::Display + std::error::Error,
{
    fn extend_error<F>(self, code: GQLErrorCode, f: F) -> async_graphql::Result<T>
    where
        F: FnOnce(&mut async_graphql::ErrorExtensionValues),
        E: std::fmt::Display,
    {
        self.map_err(|err| {
            err.extend_with(|_, e| {
                f(e);
                e.set("reason", code);
                e.set("code", code.code());
            })
        })
    }
}

/// A collection of errors code that can be used in the GraphQL schema
#[derive(Debug, Clone, Copy, showtimes_derive::EnumName)]
#[enum_name(rename_all = "snake_case")]
#[repr(u32)]
pub enum GQLErrorCode {
    // --> Common error
    /// An unknown error occurred
    UnknownError = 100,
    /// An unauthorized user
    Unauthorized = 101,
    /// An invalid request
    InvalidRequest = 102,
    /// Missing required field
    MissingRequiredField = 103,
    /// No modification to be done because of missing field
    MissingModification = 104,
    /// I/O error
    IOError = 110,
    /// Image upload error
    ImageUploadError = 120,
    /// Image delete error
    ImageDeleteError = 121,
    /// Image folder delete error
    ImageBulkDeleteError = 122,
    /// Invalid token provided
    InvalidToken = 130,
    /// Expired token provided
    ExpiredToken = 131,
    /// Internal server error
    InternalServerError = 140,

    // --> Event related
    /// Failed to advance or request next batch of events
    EventAdvanceFailure = 200,
    /// Failed to advance or request next batch of RSS events
    EventRSSAdvanceFailure = 201,

    // --> Task scheduler related
    /// Failed when requesting task scheduler
    TaskSchedulerError = 300,

    // --> Integration related
    /// Common integration error
    IntegrationError = 400,
    /// Integration not found
    IntegrationNotFound = 401,
    /// Integration already exists
    IntegrationAlreadyExists = 402,
    /// Integration is disabled for this action
    IntegrationDisabled = 403,
    /// Integration requires the original ID for updating
    IntegrationMissingOriginal = 404,

    // --> Parse related
    /// A common parsing error
    ParseError = 500,
    /// Failed to parse ULID
    ParseUlidError = 501,
    /// Failed to parse API key
    ParseAPIKeyError = 502,

    // --> Other sesison related handling
    /// Common error related to Session
    SessionError = 600,
    /// Failed to create new session
    SessionCreateError = 601,
    /// Failed to delete session
    SessionDeleteError = 602,
    /// Failed to exchange login token with Discord
    SessionExchangeError = 610,
    /// Failed to get user info from Discord
    SessionUserInfoError = 611,
    /// Failed to store session to database
    SessionStoreError = 620,
    /// Failed to store refresh session to database
    SessionRefreshStoreError = 621,

    // --> User related
    /// Failed when requesting user
    UserRequestFails = 1000,
    /// User not found in database
    UserNotFound = 1001,
    /// User already exists
    UserAlreadyExists = 1002,
    /// Invalid user audience when authenticating
    UserInvalidAudience = 1003,
    /// Unauthorized action is attempted
    UserUnauthorized = 1004,
    /// Insufficient privilege for user
    UserInsufficientPrivilege = 1005,
    /// User is owner and cannot be modified or do anything "user"-like
    UserSuperuserMode = 1006,
    /// Failed to create user
    UserCreateError = 1010,
    /// Failed to create user in search database
    UserCreateSearchError = 1011,
    /// Failed to update user
    UserUpdateError = 1012,
    /// Failed to update user in search database
    UserUpdateSearchError = 1013,
    /// Failed to delete user
    UserDeleteError = 1014,
    /// Failed to delete user in search database
    UserDeleteSearchError = 1015,
    /// Failed to create event and store it to database
    UserEventCreateError = 1016,

    // --> Server related
    /// Failed when requesting server
    ServerRequestFails = 2000,
    /// Server not found in database
    ServerNotFound = 2001,
    /// Server already exists
    ServerAlreadyExists = 2002,
    /// Failed to create server
    ServerCreateError = 2010,
    /// Failed to create server in search database
    ServerCreateSearchError = 2011,
    /// Failed to update server
    ServerUpdateError = 2012,
    /// Failed to update server in search database
    ServerUpdateSearchError = 2013,
    /// Failed to delete server
    ServerDeleteError = 2014,
    /// Failed to delete server in search database
    ServerDeleteSearchError = 2015,
    /// Server fetch disabled
    ServerFetchDisabled = 2020,

    /// Failed when requesting server premium info
    ServerPremiumRequestFails = 2100,
    /// Server premium not found in database
    ServerPremiumNotFound = 2101,
    /// Server premium already exists
    ServerPremiumAlreadyExists = 2102,
    /// Failed to create server premium data
    ServerPremiumCreateError = 2110,
    /// Failed to update server premium data
    ServerPremiumUpdateError = 2111,
    /// Failed to delete server premium data
    ServerPremiumDeleteError = 2112,

    // --> Project related
    /// Failed when requesting project
    ProjectRequestFails = 3000,
    /// Project not found in database
    ProjectNotFound = 3001,
    /// Project already exists
    ProjectAlreadyExists = 3002,
    /// Project has invalid owner
    ProjectInvalidOwner = 3003,
    /// Project is archived, (almost) no action can be done.
    ProjectArchived = 3004,
    /// Project role does not exist
    ProjectRoleNotFound = 3005,
    /// Project episodes is empty
    ProjectEmptyEpisodes = 3006,
    /// Failed to create project
    ProjectCreateError = 3010,
    /// Failed to create project in search database
    ProjectCreateSearchError = 3011,
    /// Failed to update project
    ProjectUpdateError = 3012,
    /// Failed to update project in search database
    ProjectUpdateSearchError = 3013,
    /// Failed to delete project
    ProjectDeleteError = 3014,
    /// Failed to delete project in search database
    ProjectDeleteSearchError = 3015,
    /// Project fetch disabled
    ProjectFetchDisabled = 3020,

    // --> Server collab sync related
    /// Failed when requesting server collab
    ServerSyncRequestFails = 4000,
    /// Server collab not found in database
    ServerSyncNotFound = 4001,
    /// Server collab already exists
    ServerSyncAlreadyExists = 4002,
    /// Failed to create server collab
    ServerSyncCreateError = 4010,
    /// Failed to create server collab in search database
    ServerSyncCreateSearchError = 4011,
    /// Failed to update server collab
    ServerSyncUpdateError = 4012,
    /// Failed to update server collab in search database
    ServerSyncUpdateSearchError = 4013,
    /// Failed to delete server collab
    ServerSyncDeleteError = 4014,
    /// Failed to delete server collab in search database
    ServerSyncDeleteSearchError = 4015,

    // --> Server collab ivite related
    /// Failed when requesting server collab invite
    ServerInviteRequestFails = 5000,
    /// Server collab invite not found in database
    ServerInviteNotFound = 5001,
    /// Server collab invite already exists
    ServerInviteAlreadyExists = 5002,
    /// Failed to update create collab invite
    ServerInviteCreateError = 5010,
    /// Failed to update create collab invite in search database
    ServerInviteCreateSearchError = 5011,
    /// Failed to update server collab invite
    ServerInviteUpdateError = 5012,
    /// Failed to update server collab invite in search database
    ServerInviteUpdateSearchError = 5013,
    /// Failed to delete server collab invite
    ServerInviteDeleteError = 5014,
    /// Failed to delete server collab invite in search database
    ServerInviteDeleteSearchError = 5015,

    // -> Metadata related
    /// Generic metadata error
    MetadataError = 6000,
    /// Unknown metadata source/provider
    MetadataUnknownSource = 6001,
    /// Failed to build client for metadata
    MetadataClientError = 6002,
    /// Failed to fetch poster from metadata
    MetadataPosterError = 6003,
    /// Failed when requesting metadata for Anilist
    MetadataAnilistRequestError = 6010,
    /// Failed when requesting metadata for TMDb
    #[enum_name(rename = "metadata_tmdb_request_error")]
    MetadataTMDbRequestError = 6011,
    /// Failed when requesting metadata for VNDB
    #[enum_name(rename = "metadata_vndb_request_error")]
    MetadataVNDBRequestError = 6012,
    /// Invalid ID for Anilist metadata
    MetadataAnilistInvalidId = 6020,
    /// Invalid ID for TMDb metadata
    #[enum_name(rename = "metadata_tmdb_invalid_id")]
    MetadataTMDbInvalidId = 6021,
    /// Invalid ID for VNDB metadata
    #[enum_name(rename = "metadata_vndb_invalid_id")]
    MetadataVNDBInvalidId = 6022,
    /// Metadata no episodes found
    MetadataNoEpisodesFound = 6030,
    /// Metadata unable to parse date/fuzzy date
    MetadataUnableToParseDate = 6031,
    /// Metadata no start date
    MetadataNoStartDate = 6032,

    // -> RSS related
    /// Failed when requesting RSS feed
    RSSFeedRequestFails = 7000,
    /// RSS feed not found in database
    RSSFeedNotFound = 7001,
    /// RSS feed already exists
    RSSFeedAlreadyExists = 7002,
    /// Failed to create RSS feed
    RSSFeedCreateError = 7010,
    /// Failed to create RSS feed in search database
    RSSFeedCreateSearchError = 7011,
    /// Failed to update RSS feed
    RSSFeedUpdateError = 7012,
    /// Failed to update RSS feed in search database
    RSSFeedUpdateSearchError = 7013,
    /// Failed to delete RSS feed
    RSSFeedDeleteError = 7014,
    /// Failed to delete RSS feed in search database
    RSSFeedDeleteSearchError = 7015,
    /// Failed to render RSS feed message
    RSSFeedRenderError = 7020,
    /// Failed to fetch RSS feed
    RSSFeedFetchError = 7021,
    /// RSS feed is not a valid RSS feed
    RSSFeedInvalidFeed = 7022,
}

impl GQLErrorCode {
    /// Get the error code
    pub fn code(&self) -> u32 {
        *self as u32
    }
}

/// Where an error has occured, mostly used in data loader
#[derive(Debug, Clone, Copy, showtimes_derive::EnumName)]
#[enum_name(rename_all = "snake_case")]
pub enum GQLDataLoaderWhere {
    /// User loader (ULID ID)
    UserLoaderId,
    /// User loader (Discord ID)
    UserLoaderDiscordId,
    /// User loader (API key)
    UserLoaderAPIKey,
    /// User loader db collection
    UserLoaderCollect,
    /// User loader paginated queries
    UserLoaderPaginated,
    /// User loader paginated count queries
    UserLoaderPaginatedCount,
    /// Server loader (ULID ID)
    ServerLoaderId,
    /// Server loader (Owner ID)
    ServerLoaderOwnerId,
    /// Server loader (ULID ID or Owner ID)
    ServerLoaderIdOrOwnerId,
    /// Server loader db collection
    ServerLoaderCollect,
    /// Server loader paginated queries
    ServerLoaderPaginated,
    /// Server loader paginated count queries
    ServerLoaderPaginatedCount,
    /// Project loader (ULID ID)
    ProjectLoaderId,
    /// Project loader (Owner ID)
    ProjectLoaderOwnerId,
    /// Project loader (ULID ID or Owner ID)
    ProjectLoaderCollect,
    /// Project loader paginated queries
    ProjectLoaderPaginated,
    /// Server loader paginated count queries
    ProjectLoaderPaginatedCount,
    /// Server collab loader (ULID ID)
    ServerSyncLoaderId,
    /// Server collab loader (Server ID)
    ServerSyncLoaderServerId,
    /// Server collab loader (Server ID and Project ID)
    ServerSyncLoaderServerAndProjectId,
    /// Server collab loader db collection
    ServerSyncLoaderCollect,
    /// Server collab invite loader (ULID ID)
    ServerSyncInviteLoaderId,
    /// Server collab invite loader db collection
    ServerSyncInviteLoaderCollect,
    /// RSS feed loader (ULID ID)
    RSSFeedLoaderId,
    /// RSS feed loader (Server ID)
    RSSFeedLoaderServerId,
    /// RSS feed loader db collection
    RSSFeedLoaderCollect,
}

impl From<GQLErrorCode> for async_graphql::Value {
    fn from(value: GQLErrorCode) -> Self {
        async_graphql::Value::String(value.to_name().to_string())
    }
}

impl From<GQLDataLoaderWhere> for async_graphql::Value {
    fn from(value: GQLDataLoaderWhere) -> Self {
        async_graphql::Value::String(value.to_name().to_string())
    }
}

/// The error wrapping [`Error`]
///
/// This is a builder to help make a proper error extension result for the API.
#[derive(Clone)]
pub struct GQLError {
    message: String,
    code: GQLErrorCode,
    loader: Option<GQLDataLoaderWhere>,
    extensions: async_graphql::ErrorExtensionValues,
    loader_inner: Option<GQLDataLoaderWhere>,
}

impl GQLError {
    /// Create a new [`GQLError`]
    pub fn new(message: impl Into<String>, code: GQLErrorCode) -> Self {
        Self {
            message: message.into(),
            code,
            loader: None,
            extensions: async_graphql::ErrorExtensionValues::default(),
            loader_inner: None,
        }
    }

    /// Set the error code
    pub fn code(mut self, code: GQLErrorCode) -> Self {
        self.code = code;
        self
    }

    /// Extend the extensions with a closure
    ///
    /// You can set anything you want in the [`async_graphql::ErrorExtensionValues`].
    ///
    /// # Note
    /// * You will be unable to replace `code`, `reason`, `where` and `where_req` fields.
    pub fn extend(mut self, f: impl FnOnce(&mut async_graphql::ErrorExtensionValues)) -> Self {
        f(&mut self.extensions);
        self
    }

    /// Set the [`GQLDataLoaderWhere`] that caused the error
    pub fn loader(mut self, loader: GQLDataLoaderWhere) -> Self {
        self.loader = Some(loader);
        self
    }

    /// Set the [`GQLDataLoaderWhere`] that caused the error.
    ///
    /// Usually used as an extension of [`GQLError::loader`].
    /// This will be ignored if [`GQLError::loader`] not set.
    pub fn inner_loader(mut self, loader_inner: GQLDataLoaderWhere) -> Self {
        self.loader_inner = Some(loader_inner);
        self
    }

    /// Build the [`Error`]
    pub fn build(mut self) -> Error {
        let mut errors = Error::new(self.message);
        self.extensions.set("code", self.code.code());
        self.extensions.set("reason", self.code);
        match (self.loader, self.loader_inner) {
            (Some(loader), Some(loader_inner)) => {
                self.extensions.set("where", loader);
                self.extensions.set("where_req", loader_inner);
            }
            (Some(loader), None) => {
                self.extensions.set("where", loader);
            }
            _ => {}
        }
        errors.extensions = Some(self.extensions);
        errors
    }
}

impl From<GQLError> for Error {
    fn from(value: GQLError) -> Self {
        value.build()
    }
}

impl<T> From<GQLError> for async_graphql::Result<T> {
    fn from(value: GQLError) -> Self {
        Err(value.build())
    }
}
