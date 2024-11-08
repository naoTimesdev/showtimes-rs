//! Some request guard for GraphQL

use async_graphql::Guard;
use showtimes_session::ShowtimesUserSession;

use crate::{data_loader::find_authenticated_user, errors::GQLError, GQLErrorCode, UserKindGQL};

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
                                GQLError::new(
                                    "Missing privilege",
                                    GQLErrorCode::UserInsufficientPrivilege,
                                )
                                .extend(|e| {
                                    e.set("id", user.id.to_string());
                                    e.set("required", self.level.to_name());
                                    e.set("current", user.kind.to_name());
                                })
                                .into()
                            }
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    // Ignore the user level check
                    Ok(())
                }
            }
            None => GQLError::new("Unauthorized", GQLErrorCode::Unauthorized).into(),
        }
    }
}
