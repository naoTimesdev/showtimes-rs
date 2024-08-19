use async_graphql::extensions::Tracing;
use async_graphql::{Context, EmptySubscription, Error, ErrorExtensions, Object};
use models::users::UserSessionGQL;
use showtimes_db::{mongodb::bson::doc, DatabaseMutex};
use showtimes_session::{oauth2::discord::DiscordClient, ShowtimesUserSession};
use std::sync::Arc;

mod models;

pub type ShowtimesGQLSchema = async_graphql::Schema<QueryRoot, MutationRoot, EmptySubscription>;
pub use async_graphql::http::playground_source;
pub use async_graphql::http::GraphQLPlaygroundConfig;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn current<'a>(&self, ctx: &'a Context<'_>) -> async_graphql::Result<UserSessionGQL> {
        match ctx.data_opt::<ShowtimesUserSession>() {
            Some(session) => {
                let handler =
                    showtimes_db::UserHandler::new(ctx.data_unchecked::<DatabaseMutex>().clone())
                        .await;

                let user = handler
                    .find_by(doc! { "id": session.get_claims().get_metadata() })
                    .await?;

                match user {
                    Some(user) => Ok(UserSessionGQL::new(user, session.get_token())),
                    None => Err(Error::new("User not found")
                        .extend_with(|_, e| e.set("reason", "not_found"))),
                }
            }
            None => {
                Err(Error::new("Unauthorized").extend_with(|_, e| e.set("reason", "unauthorized")))
            }
        }
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn auth<'a>(
        &self,
        ctx: &'a Context<'_>,
        token: String,
        state: String,
    ) -> async_graphql::Result<UserSessionGQL> {
        let config = ctx.data_unchecked::<Arc<showtimes_shared::Config>>();

        tracing::info!("Authenticating user with token: {}", &token);
        showtimes_session::verify_discord_session_state(&state, &config.jwt.secret).map_err(
            |err| {
                err.extend_with(|_, e| {
                    e.set("reason", "invalid_state");
                    e.set("state", state);
                    e.set("token", token.clone());
                })
            },
        )?;

        // Valid!
        let discord = ctx.data_unchecked::<Arc<DiscordClient>>();

        tracing::info!("Exchanging code {} for OAuth2 token...", &token);
        let exchanged = discord
            .exchange_code(&token, &config.discord.redirect_url)
            .await?;

        tracing::info!("Success, getting user for code {}", &token);
        let user_info = discord.get_user(&exchanged.access_token).await?;

        let handler =
            showtimes_db::UserHandler::new(ctx.data_unchecked::<DatabaseMutex>().clone()).await;

        tracing::info!("Checking if user exists for ID: {}", &user_info.id);
        let user = handler
            .find_by(doc! { "discord_meta.id": user_info.id.clone() })
            .await?;

        match user {
            Some(mut user) => {
                tracing::info!("User found, updating token for ID: {}", &user_info.id);
                // Update the user token
                user.discord_meta.access_token = exchanged.access_token;
                user.discord_meta.refresh_token = exchanged.refresh_token.unwrap();
                user.discord_meta.expires_at =
                    chrono::Utc::now().timestamp() + exchanged.expires_in as i64;

                if !user.registered {
                    user.discord_meta.username = user_info.username.clone();
                    user.registered = true;
                }

                handler.save(&mut user, None).await?;

                let oauth_token = showtimes_session::create_session(
                    user.id,
                    config.jwt.expiration.unwrap_or(7 * 24 * 60 * 60),
                    &config.jwt.secret,
                )?;
                Ok(UserSessionGQL::new(user, oauth_token))
            }
            None => {
                tracing::info!(
                    "User not found, creating new user for ID: {}",
                    &user_info.id
                );
                // Create new user
                let current_time = chrono::Utc::now();
                let expires_at = current_time.timestamp() + exchanged.expires_in as i64;
                let discord_user = showtimes_db::m::DiscordUser {
                    id: user_info.id,
                    username: user_info.username.clone(),
                    avatar: user_info.avatar,
                    access_token: exchanged.access_token,
                    refresh_token: exchanged.refresh_token.unwrap(),
                    expires_at,
                };

                let mut user = showtimes_db::m::User::new(user_info.username, discord_user);
                handler.save(&mut user, None).await?;

                let oauth_token = showtimes_session::create_session(
                    user.id,
                    config.jwt.expiration.unwrap_or(7 * 24 * 60 * 60),
                    &config.jwt.secret,
                )?;
                Ok(UserSessionGQL::new(user, oauth_token))
            }
        }
    }
}

/// Create the GraphQL schema
pub fn create_schema() -> ShowtimesGQLSchema {
    async_graphql::Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .extension(Tracing)
        .finish()
}
