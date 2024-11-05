//! Some request guard for GraphQL

use async_graphql::{ErrorExtensions, Guard};
use showtimes_session::ShowtimesUserSession;

use crate::{data_loader::find_authenticated_user, GQLError, UserKindGQL};

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
        match ctx.data_opt::<ShowtimesUserSession>() {
            Some(session) => {
                if self.level >= UserKindGQL::Admin {
                    tracing::info!(
                        "Checking user level for admin for session: {}",
                        session.get_claims().get_metadata()
                    );

                    let user = find_authenticated_user(ctx).await;

                    match user {
                        Ok(user) => {
                            if UserKindGQL::from(user.kind) >= self.level {
                                Ok(())
                            } else {
                                Err(async_graphql::Error::new("Missing privilege").extend_with(
                                    |_, e| {
                                        e.set("id", user.id.to_string());
                                        e.set("required", self.level.to_name());
                                        e.set("current", user.kind.to_name());
                                        e.set("reason", GQLError::UserInsufficientPrivilege);
                                        e.set("code", GQLError::UserInsufficientPrivilege.code());
                                    },
                                ))
                            }
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    // Ignore the user level check
                    Ok(())
                }
            }
            None => Err(
                async_graphql::Error::new("Unauthorized").extend_with(|_, e| {
                    e.set("reason", GQLError::Unauthorized);
                    e.set("code", GQLError::Unauthorized.code());
                }),
            ),
        }
    }
}
