//! Some request guard for GraphQL

use async_graphql::Guard;
use showtimes_session::ShowtimesUserSession;

use crate::{GQLErrorCode, GQLErrorExt, UserKindGQL, errors::GQLError};

/// A guard to check if the user is at least a certain level
pub struct AuthUserMinimumGuard {
    level: UserKindGQL,
}

impl AuthUserMinimumGuard {
    /// Create a new auth user minimum guard
    pub fn new(level: UserKindGQL) -> Self {
        Self { level }
    }
}

impl Guard for AuthUserMinimumGuard {
    async fn check(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<()> {
        match ctx.data_opt::<showtimes_db::m::User>() {
            Some(user) => {
                if UserKindGQL::from(user.kind) >= self.level {
                    Ok(())
                } else {
                    GQLError::new("Missing privilege", GQLErrorCode::UserInsufficientPrivilege)
                        .extend(|e| {
                            e.set("id", user.id.to_string());
                            e.set("required", self.level.to_name());
                            e.set("current", user.kind.to_name());
                        })
                        .into()
                }
            }
            None => GQLError::new("Unauthorized", GQLErrorCode::Unauthorized).into(),
        }
    }
}

/// The verification method for the API key
#[derive(Clone, Copy)]
pub enum APIKeyVerify {
    /// Any capability
    Any(&'static [showtimes_db::m::APIKeyCapability]),
    /// All capabilities
    All(&'static [showtimes_db::m::APIKeyCapability]),
    /// Specific capability
    Specific(showtimes_db::m::APIKeyCapability),
    /// Do not allow API key
    NotAllowed,
    /// Allow any API key
    AllowAny,
}

/// A guard to check if the user API key session is on minimum permissions level
pub struct AuthAPIKeyMinimumGuard {
    permissions: APIKeyVerify,
}

impl AuthAPIKeyMinimumGuard {
    /// Create a new auth user minimum guard
    pub fn new(permissions: APIKeyVerify) -> Self {
        Self { permissions }
    }
}

impl Guard for AuthAPIKeyMinimumGuard {
    async fn check(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<()> {
        match (
            ctx.data_opt::<showtimes_db::m::User>(),
            ctx.data_opt::<ShowtimesUserSession>(),
        ) {
            (Some(user), Some(session)) => {
                if session.get_claims().get_audience()
                    != showtimes_session::ShowtimesAudience::APIKey
                {
                    // Not API key session
                    return Ok(());
                }

                // load as API key
                let api_key = session.get_claims().get_metadata();
                let parse_api = showtimes_shared::APIKey::try_from(api_key).extend_error(
                    GQLErrorCode::ParseAPIKeyError,
                    |e| {
                        e.set("user", user.id.to_string());
                        e.set("value", api_key);
                        e.set(
                            "audience",
                            showtimes_session::ShowtimesAudience::APIKey.to_string(),
                        );
                    },
                )?;

                // Find API key
                let match_key = user
                    .api_key
                    .iter()
                    .find(|&k| k.key == parse_api)
                    .ok_or_else(|| {
                        GQLError::new(
                            "API key not found in the user list",
                            GQLErrorCode::APIKeyNotFound,
                        )
                        .extend(|e| {
                            e.set("user", user.id.to_string());
                            e.set("key", parse_api.to_string());
                        })
                    })?;

                match self.permissions {
                    APIKeyVerify::Any(capabilities) => {
                        if match_key.can_any(capabilities) {
                            Ok(())
                        } else {
                            Err(GQLError::new(
                                "API key does not have any of the required capabilities",
                                GQLErrorCode::APIKeyMissingCapability,
                            )
                            .extend(|e| {
                                e.set("user", user.id.to_string());
                                e.set("key", parse_api.to_string());
                                e.set(
                                    "required",
                                    capabilities.iter().map(|c| c.to_name()).collect::<Vec<_>>(),
                                );
                                e.set(
                                    "current",
                                    match_key
                                        .capabilities
                                        .iter()
                                        .map(|c| c.to_name())
                                        .collect::<Vec<_>>(),
                                );
                                e.set("mode", "any");
                            })
                            .build())
                        }
                    }
                    APIKeyVerify::All(capabilities) => {
                        if match_key.can_all(capabilities) {
                            Ok(())
                        } else {
                            Err(GQLError::new(
                                "API key does not have all required capabilities",
                                GQLErrorCode::APIKeyMissingCapability,
                            )
                            .extend(|e| {
                                e.set("user", user.id.to_string());
                                e.set("key", parse_api.to_string());
                                e.set(
                                    "required",
                                    capabilities.iter().map(|c| c.to_name()).collect::<Vec<_>>(),
                                );
                                e.set(
                                    "current",
                                    match_key
                                        .capabilities
                                        .iter()
                                        .map(|c| c.to_name())
                                        .collect::<Vec<_>>(),
                                );
                                e.set("mode", "all");
                            })
                            .build())
                        }
                    }
                    APIKeyVerify::Specific(capability) => {
                        if match_key.can(capability) {
                            Ok(())
                        } else {
                            Err(GQLError::new(
                                "API key does not have the required capability",
                                GQLErrorCode::APIKeyMissingCapability,
                            )
                            .extend(|e| {
                                e.set("user", user.id.to_string());
                                e.set("key", parse_api.to_string());
                                e.set("capability", capability.to_name());
                                e.set(
                                    "current",
                                    match_key
                                        .capabilities
                                        .iter()
                                        .map(|c| c.to_name())
                                        .collect::<Vec<_>>(),
                                );
                                e.set("mode", "specific");
                            })
                            .build())
                        }
                    }
                    APIKeyVerify::AllowAny => Ok(()),
                    APIKeyVerify::NotAllowed => Err(GQLError::new(
                        "API key is not allowed for this operation",
                        GQLErrorCode::APIKeyNotAllowed,
                    )
                    .extend(|e| {
                        e.set("user", user.id.to_string());
                        e.set("key", parse_api.to_string());
                    })
                    .build()),
                }
            }
            _ => GQLError::new("Unauthorized", GQLErrorCode::Unauthorized).into(),
        }
    }
}

/// A guard to check both user and API key permissions
pub struct AuthUserAndAPIKeyGuard {
    level: UserKindGQL,
    permissions: APIKeyVerify,
}

impl AuthUserAndAPIKeyGuard {
    /// Create a new auth user and API key guard
    pub fn new(level: UserKindGQL, permissions: APIKeyVerify) -> Self {
        Self { level, permissions }
    }
}

impl Guard for AuthUserAndAPIKeyGuard {
    async fn check(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<()> {
        let guard_user_auth = AuthUserMinimumGuard::new(self.level);
        let guard_api_key = AuthAPIKeyMinimumGuard::new(self.permissions);

        guard_user_auth.check(ctx).await?;
        guard_api_key.check(ctx).await
    }
}

/// Only show this field to admin
pub fn visible_minimum_admin(ctx: &async_graphql::Context<'_>) -> bool {
    match ctx.data_opt::<showtimes_db::m::User>() {
        Some(user) => UserKindGQL::from(user.kind) >= UserKindGQL::Admin,
        None => false,
    }
}
