use async_graphql::Guard;
use showtimes_db::{m::UserKind, mongodb::bson::doc, DatabaseMutex};
use showtimes_session::ShowtimesUserSession;

#[derive(Eq, PartialEq, Copy, Clone, PartialOrd, Ord)]
pub enum AuthLevel {
    User,
    Admin,
}

pub struct AuthUserMinimumGuard {
    level: AuthLevel,
}

impl AuthUserMinimumGuard {
    pub fn new(level: AuthLevel) -> Self {
        Self { level }
    }
}

impl From<UserKind> for AuthLevel {
    fn from(value: UserKind) -> Self {
        match value {
            UserKind::User => AuthLevel::User,
            UserKind::Admin => AuthLevel::Admin,
        }
    }
}

impl Guard for AuthUserMinimumGuard {
    async fn check(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<()> {
        match ctx.data_opt::<ShowtimesUserSession>() {
            Some(session) => {
                if self.level >= AuthLevel::Admin {
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
                            if AuthLevel::from(user.kind) >= self.level {
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
