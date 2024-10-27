//! A collection of errors mainly used in the GraphQL schema to return as code.

#[derive(Debug, Clone, Copy, showtimes_derive::EnumName)]
#[enum_name(rename_all = "snake_case")]
#[repr(u32)]
pub enum GQLError {
    // Common error
    UnknownError = 100,
    Unauthorized = 101,
    InvalidRequest = 102,
    MissingRequiredField = 103,

    // User related
    UserRequestFails = 1000,
    UserNotFound = 1001,
    UserAlreadyExists = 1002,
    UserInvalidAudience = 1003,
    UserUnauthorized = 1004,
    UserInsufficientPrivilege = 1005,

    // Server related
    ServerRequestFails = 2000,
    ServerNotFound = 2001,
    ServerAlreadyExists = 2002,
    ProjectRequestFails = 3000,

    // Project related
    ProjectNotFound = 3001,
    ProjectAlreadyExists = 3002,
    ServerSyncRequestFails = 4000,

    // Server sync collab related
    ServerSyncNotFound = 4001,
    ServerSyncAlreadyExists = 4002,
}

impl GQLError {
    /// Get the error code
    pub fn code(&self) -> u32 {
        *self as u32
    }
}

#[derive(Debug, Clone, Copy, showtimes_derive::EnumName)]
#[enum_name(rename_all = "snake_case")]
pub enum GQLDataLoaderWhere {
    UserLoaderId,
    UserLoaderDiscordId,
    UserLoaderAPIKey,
    UserLoaderCollect,
    ServerLoaderId,
    ServerLoaderOwnerId,
    ServerLoaderIdOrOwnerId,
    ServerLoaderCollect,
    ProjectLoaderId,
    ProjectLoaderOwnerId,
    ProjectLoaderCollect,
    ServerSyncLoaderId,
    ServerSyncLoaderServerId,
    ServerSyncLoaderServerAndProjectId,
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
