use async_graphql::{Context, EmptySubscription, Error, ErrorExtensions, Object};
use models::users::UserSessionGQL;
use showtimes_db::{mongodb::bson::doc, DatabaseMutex};
use showtimes_session::{oauth2::discord::DiscordClient, ShowtimesUserSession};
use std::sync::Arc;

mod models;

pub type ShowtimesGQLSchema = async_graphql::Schema<QueryRoot, MutationRoot, EmptySubscription>;

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
        let config = ctx.data_unchecked::<showtimes_shared::Config>();

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

        let exchanged = discord
            .exchange_code(token, &config.discord.redirect_url)
            .await?;

        let user_info = discord.get_user(&exchanged.access_token).await?;

        let handler =
            showtimes_db::UserHandler::new(ctx.data_unchecked::<DatabaseMutex>().clone()).await;

        let user = handler
            .find_by(doc! { "discord.id": user_info.id.clone() })
            .await?;

        match user {
            Some(mut user) => {
                // Update the user token
                user.discord_meta.access_token = exchanged.access_token;
                user.discord_meta.refresh_token = exchanged.refresh_token.unwrap();
                handler.save(&mut user, None).await?;

                let oauth_token = showtimes_session::create_session(
                    user.id,
                    config.jwt.expiration.unwrap_or(7 * 24 * 60 * 60),
                    &config.jwt.secret,
                )?;
                Ok(UserSessionGQL::new(user, oauth_token))
            }
            None => {
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
