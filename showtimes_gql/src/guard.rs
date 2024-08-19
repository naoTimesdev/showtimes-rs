use async_graphql::Guard;
use showtimes_db::{mongodb::bson::doc, DatabaseMutex};
use showtimes_session::ShowtimesUserSession;

use crate::models::users::UserKindGQL;

pub struct AuthUserMinimumGuard {
    level: UserKindGQL,
}

impl AuthUserMinimumGuard {
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

                    let handler = showtimes_db::UserHandler::new(
                        ctx.data_unchecked::<DatabaseMutex>().clone(),
                    )
                    .await;

                    let user = handler
                        .find_by(doc! { "id": session.get_claims().get_metadata() })
                        .await?;

                    match user {
                        Some(user) => {
                            if UserKindGQL::from(user.kind) >= self.level {
                                Ok(())
                            } else {
                                Err("User level not authorized".into())
                            }
                        }
                        None => Err("Unknown account in OAuth2 token".into()),
                    }
                } else {
                    // Ignore the user level check
                    Ok(())
                }
            }
            None => Err("Unauthorized".into()),
        }
    }
}
