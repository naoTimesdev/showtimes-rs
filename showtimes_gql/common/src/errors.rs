//! A collection of errors mainly used in the GraphQL schema to return as code.

/// A collection of errors code that can be used in the GraphQL schema
#[derive(Debug, Clone, Copy, showtimes_derive::EnumName)]
#[enum_name(rename_all = "snake_case")]
#[repr(u32)]
pub enum GQLError {
    // --> Common error
    /// An unknown error occurred
    UnknownError = 100,
    /// An unauthorized user
    Unauthorized = 101,
    /// An invalid request
    InvalidRequest = 102,
    /// Missing required field
    MissingRequiredField = 103,
    /// I/O error
    IOError = 110,
    /// Image upload error
    ImageUploadError = 120,

    // --> Event related
    /// Failed to advance or request next batch of events
    EventAdvanceFailure = 200,

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
}

impl GQLError {
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

impl From<GQLError> for async_graphql::Value {
    fn from(value: GQLError) -> Self {
        async_graphql::Value::String(value.to_name().to_string())
    }
}

impl From<GQLDataLoaderWhere> for async_graphql::Value {
    fn from(value: GQLDataLoaderWhere) -> Self {
        async_graphql::Value::String(value.to_name().to_string())
    }
}
