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
    /// Invalid token provided
    InvalidToken = 130,
    /// Expired token provided
    ExpiredToken = 131,

    // --> Event related
    /// Failed to advance or request next batch of events
    EventAdvanceFailure = 200,

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

    // --> Server related
    /// Failed when requesting server
    ServerRequestFails = 2000,
    /// Server not found in database
    ServerNotFound = 2001,
    /// Server already exists
    ServerAlreadyExists = 2002,

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

    // --> Server collab sync related
    /// Failed when requesting server collab
    ServerSyncRequestFails = 4000,
    /// Server collab not found in database
    ServerSyncNotFound = 4001,
    /// Server collab already exists
    ServerSyncAlreadyExists = 4002,

    // --> Server collab ivite related
    /// Failed when requesting server collab invite
    ServerInviteRequestFails = 5000,
    /// Server collab invite not found in database
    ServerInviteNotFound = 5001,
    /// Server collab invite already exists
    ServerInviteAlreadyExists = 5002,

    // -> Metadata related
    /// Generic metadata error
    MetadataError = 6000,
    /// Unknown metadata source/provider
    MetadataUnknownSource = 6001,
    /// Failed when requesting metadata for Anilist
    MetadataAnilistRequestError = 6010,
    /// Failed when requesting metadata for TMDb
    MetadataTMDbRequestError = 6011,
    /// Failed when requesting metadata for VNDB
    MetadataVNDBRequestError = 6012,
    /// Invalid ID for Anilist metadata
    MetadataAnilistInvalidId = 6020,
    /// Invalid ID for TMDb metadata
    MetadataTMDbInvalidId = 6021,
    /// Invalid ID for VNDB metadata
    MetadataVNDBInvalidId = 6022,
    /// Metadata no episodes found
    MetadataNoEpisodesFound = 6030,
    /// Metadata unable to parse date/fuzzy date
    MetadataUnableToParseDate = 6031,
    /// Metadata no start date
    MetadataNoStartDate = 6032,
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
    /// Server loader (ULID ID)
    ServerLoaderId,
    /// Server loader (Owner ID)
    ServerLoaderOwnerId,
    /// Server loader (ULID ID or Owner ID)
    ServerLoaderIdOrOwnerId,
    /// Server loader db collection
    ServerLoaderCollect,
    /// Project loader (ULID ID)
    ProjectLoaderId,
    /// Project loader (Owner ID)
    ProjectLoaderOwnerId,
    /// Project loader (ULID ID or Owner ID)
    ProjectLoaderCollect,
    /// Server collab loader (ULID ID)
    ServerSyncLoaderId,
    /// Server collab loader (Server ID)
    ServerSyncLoaderServerId,
    /// Server collab loader (Server ID and Project ID)
    ServerSyncLoaderServerAndProjectId,
    /// Server collab loader db collection
    ServerSyncLoaderCollect,
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
