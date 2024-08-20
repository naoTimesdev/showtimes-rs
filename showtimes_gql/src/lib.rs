use async_graphql::dataloader::DataLoader;
use async_graphql::extensions::Tracing;
use async_graphql::{Context, EmptySubscription, Error, ErrorExtensions, Object};
use data_loader::{DiscordIdLoad, UserDataLoader};
use futures::TryStreamExt;
use models::prelude::{PageInfoGQL, PaginatedGQL, UlidGQL};
use models::servers::ServerGQL;
use models::users::UserSessionGQL;
use showtimes_db::{mongodb::bson::doc, DatabaseShared};
use showtimes_session::{oauth2::discord::DiscordClient, ShowtimesUserSession};
use std::sync::Arc;

mod data_loader;
mod guard;
mod models;

pub type ShowtimesGQLSchema = async_graphql::Schema<QueryRoot, MutationRoot, EmptySubscription>;
pub use async_graphql::http::{graphiql_plugin_explorer, GraphiQLSource};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get current authenticated user
    #[graphql(guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)")]
    async fn current<'a>(&self, ctx: &'a Context<'_>) -> async_graphql::Result<UserSessionGQL> {
        let user_session = ctx.data_unchecked::<ShowtimesUserSession>();
        let handler = showtimes_db::UserHandler::new(ctx.data_unchecked::<DatabaseShared>());

        let user = handler
            .find_by(doc! { "id": user_session.get_claims().get_metadata() })
            .await?;

        match user {
            Some(user) => Ok(UserSessionGQL::new(user, user_session.get_token())),
            None => {
                Err(Error::new("User not found").extend_with(|_, e| e.set("reason", "not_found")))
            }
        }
    }

    /// Get authenticated user associated servers
    #[graphql(guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)")]
    async fn servers<'a>(
        &self,
        ctx: &'a Context<'_>,
        #[graphql(desc = "Limit what server we want to return")] ids: Option<Vec<UlidGQL>>,
        #[graphql(desc = "The number of servers to return, default 20", name = "perPage")] per_page: Option<
            u32,
        >,
        #[graphql(desc = "The cursor to start from")] cursor: Option<UlidGQL>,
    ) -> async_graphql::Result<PaginatedGQL<ServerGQL>> {
        let user_session = ctx.data_unchecked::<ShowtimesUserSession>();
        let user_id = user_session.get_claims().get_metadata();
        let db = ctx.data_unchecked::<DatabaseShared>();

        // Allowed range of per_page is 10-100, with
        let per_page = per_page.filter(|&p| (2..=100).contains(&p)).unwrap_or(20);

        let srv_handler = showtimes_db::ServerHandler::new(db);

        let doc_query = match (cursor, ids) {
            (Some(cursor), Some(ids)) => {
                let ids: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();
                doc! {
                    "owners.id": user_id.to_string(),
                    "id": { "$gte": cursor.to_string(), "$in": ids }
                }
            }
            (Some(cursor), None) => {
                doc! {
                    "owners.id": user_id.to_string(),
                    "id": { "$gte": cursor.to_string() }
                }
            }
            (None, Some(ids)) => {
                let ids: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();
                doc! {
                    "owners.id": user_id.to_string(),
                    "id": { "$in": ids }
                }
            }
            (None, None) => doc! { "owners.id": user_id.to_string() },
        };

        let cursor = srv_handler
            .get_collection()
            .find(doc_query)
            .limit((per_page + 1) as i64)
            .sort(doc! { "id": 1 })
            .await?;
        let count = srv_handler
            .get_collection()
            .count_documents(doc! { "owners.id": user_id.to_string() })
            .await?;

        let mut all_servers: Vec<showtimes_db::m::Server> = cursor.try_collect().await?;

        // If all_servers is equal to per_page, then there is a next page
        let last_srv = if all_servers.len() == per_page as usize {
            Some(all_servers.pop().unwrap())
        } else {
            None
        };

        let page_info = PageInfoGQL::new(count, per_page, last_srv.map(|p| p.id.into()));

        Ok(PaginatedGQL::new(
            all_servers.into_iter().map(|p| p.into()).collect(),
            page_info,
        ))
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Authorize Discord OAuth2 token and state that was returned from the OAuth2 redirect
    async fn auth<'a>(
        &self,
        ctx: &'a Context<'_>,
        #[graphql(desc = "The OAuth2 token/code returned from Discord")] token: String,
        #[graphql(desc = "The OAuth2 state")] state: String,
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

        // Load handler and data loader
        let handler = showtimes_db::UserHandler::new(ctx.data_unchecked::<DatabaseShared>());
        let loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();

        tracing::info!("Checking if user exists for ID: {}", &user_info.id);
        let user = loader.load_one(DiscordIdLoad(user_info.id.clone())).await?;

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
pub fn create_schema(db_pool: &DatabaseShared) -> ShowtimesGQLSchema {
    async_graphql::Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .extension(Tracing)
        .data(DataLoader::new(UserDataLoader::new(db_pool), tokio::spawn))
        .finish()
}
