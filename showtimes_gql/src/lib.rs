use async_graphql::dataloader::DataLoader;
use async_graphql::extensions::Tracing;
use async_graphql::{Context, EmptySubscription, ErrorExtensions, Object};
use data_loader::{
    find_authenticated_user, DiscordIdLoad, ServerAndOwnerId, ServerDataLoader, ServerOwnerId,
    UserDataLoader,
};
use models::prelude::{OkResponse, PaginatedGQL};
use models::projects::ProjectGQL;
use models::search::QuerySearchRoot;
use models::servers::ServerGQL;
use models::stats::StatsGQL;
use models::users::{UserGQL, UserSessionGQL};
use showtimes_db::{mongodb::bson::doc, DatabaseShared};
use showtimes_session::manager::SharedSessionManager;
use showtimes_session::{oauth2::discord::DiscordClient, ShowtimesUserSession};
use std::sync::Arc;

mod data_loader;
mod guard;
mod image;
mod models;
mod mutations;
mod queries;

pub type ShowtimesGQLSchema = async_graphql::Schema<QueryRoot, MutationRoot, EmptySubscription>;
pub use async_graphql::http::{graphiql_plugin_explorer, GraphiQLSource};
pub use image::MAX_IMAGE_SIZE;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get current authenticated user
    #[graphql(guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)")]
    async fn current<'a>(&self, ctx: &'a Context<'_>) -> async_graphql::Result<UserSessionGQL> {
        let user_session = ctx.data_unchecked::<ShowtimesUserSession>();
        let user = find_authenticated_user(ctx).await?;

        Ok(UserSessionGQL::new(user, user_session.get_token()))
    }

    /// Get authenticated user associated servers
    #[graphql(guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)")]
    async fn servers(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Specify server IDs to query")] ids: Option<
            Vec<crate::models::prelude::UlidGQL>,
        >,
        #[graphql(
            name = "perPage",
            desc = "The number of servers to return, default to 20",
            validator(minimum = 2, maximum = 100)
        )]
        per_page: Option<u32>,
        #[graphql(desc = "The cursor to start from")] cursor: Option<
            crate::models::prelude::UlidGQL,
        >,
        #[graphql(desc = "Sort order, default to ID_ASC")] sort: Option<
            models::prelude::SortOrderGQL,
        >,
    ) -> async_graphql::Result<PaginatedGQL<ServerGQL>> {
        let user = find_authenticated_user(ctx).await?;
        let mut queries = queries::servers::ServerQuery::new()
            .with_current_user(queries::ServerQueryUser::from(&user));
        if let Some(ids) = ids {
            queries.set_ids(ids.into_iter().map(|id| *id).collect());
        };
        if let Some(per_page) = per_page {
            queries.set_per_page(per_page);
        }
        if let Some(cursor) = cursor {
            queries.set_cursor(*cursor);
        }
        if let Some(sort) = sort {
            queries.set_sort(sort);
        }

        let results = queries::servers::query_servers_paginated(ctx, queries).await?;

        Ok(results)
    }

    /// Get authenticated user associated projects
    #[graphql(guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)")]
    #[allow(clippy::too_many_arguments)]
    async fn projects(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Specify project IDs to query")] ids: Option<
            Vec<crate::models::prelude::UlidGQL>,
        >,
        #[graphql(name = "serverIds", desc = "Limit projects to specific server IDs")]
        server_ids: Option<Vec<crate::models::prelude::UlidGQL>>,
        #[graphql(
            name = "perPage",
            desc = "The number of project to return, default to 20",
            validator(minimum = 2, maximum = 100)
        )]
        per_page: Option<u32>,
        #[graphql(desc = "The cursor to start from")] cursor: Option<
            crate::models::prelude::UlidGQL,
        >,
        #[graphql(desc = "Sort order, default to ID_ASC")] sort: Option<
            models::prelude::SortOrderGQL,
        >,
        #[graphql(desc = "Remove pagination limit, this only works if you're an Admin")]
        unpaged: bool,
    ) -> async_graphql::Result<PaginatedGQL<ProjectGQL>> {
        let user = find_authenticated_user(ctx).await?;
        let allowed_servers = match user.kind {
            showtimes_db::m::UserKind::User => {
                let projector = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();

                projector.load_one(ServerOwnerId::new(user.id)).await?
            }
            _ => None,
        };

        let mut queries =
            queries::projects::ProjectQuery::new().with_current_user(user.clone().into());
        if let Some(ids) = ids {
            queries.set_ids(ids.into_iter().map(|id| *id).collect());
        };
        if let Some(server_ids) = server_ids {
            let server_ids: Vec<showtimes_shared::ulid::Ulid> =
                server_ids.into_iter().map(|id| *id).collect();
            queries.set_creators(&server_ids);
        };
        if let Some(per_page) = per_page {
            queries.set_per_page(per_page);
        }
        if let Some(cursor) = cursor {
            queries.set_cursor(*cursor);
        }
        if let Some(sort) = sort {
            queries.set_sort(sort);
        }
        if let Some(allowed_servers) = allowed_servers {
            queries.set_allowed_servers(allowed_servers);
        }
        if unpaged && user.kind != showtimes_db::m::UserKind::User {
            queries.set_unpaged();
        }

        let results = queries::projects::query_projects_paginated(ctx, queries).await?;

        Ok(results)
    }

    /// Get all available users, you need a minimum of admin role to access this
    #[graphql(guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)")]
    async fn users(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Specify user IDs to query")] ids: Option<
            Vec<crate::models::prelude::UlidGQL>,
        >,
        #[graphql(
            name = "perPage",
            desc = "The number of users to return, default to 20",
            validator(minimum = 2, maximum = 100)
        )]
        per_page: Option<u32>,
        #[graphql(desc = "The cursor to start from")] cursor: Option<
            crate::models::prelude::UlidGQL,
        >,
        #[graphql(desc = "Sort order, default to ID_ASC")] sort: Option<
            models::prelude::SortOrderGQL,
        >,
    ) -> async_graphql::Result<PaginatedGQL<UserGQL>> {
        let user = find_authenticated_user(ctx).await?;
        let mut queries = queries::users::UserQuery::new()
            .with_current_user(queries::ServerQueryUser::from(&user));
        if let Some(ids) = ids {
            queries.set_ids(ids.into_iter().map(|id| *id).collect());
        };
        if let Some(per_page) = per_page {
            queries.set_per_page(per_page);
        }
        if let Some(cursor) = cursor {
            queries.set_cursor(*cursor);
        }
        if let Some(sort) = sort {
            queries.set_sort(sort);
        }

        let results = queries::users::query_users_paginated(ctx, queries).await?;

        Ok(results)
    }

    /// Get server statistics
    #[graphql(guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)")]
    async fn stats(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Specify server ID to query")] id: crate::models::prelude::UlidGQL,
    ) -> async_graphql::Result<StatsGQL> {
        let user = find_authenticated_user(ctx).await?;

        let projector = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();
        let result = match user.kind {
            showtimes_db::m::UserKind::User => {
                projector
                    .load_one(ServerAndOwnerId::new(*id, user.id))
                    .await?
            }
            _ => projector.load_one(*id).await?,
        };

        match result {
            Some(server) => Ok(StatsGQL::new(server)),
            None => Err("Server not found".into()),
        }
    }

    /// Do a external searvice metadata search
    #[graphql(guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)")]
    async fn search(&self) -> QuerySearchRoot {
        // This is just an empty root which has dynamic fields
        QuerySearchRoot::new()
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
        let sess_manager = ctx.data_unchecked::<SharedSessionManager>();

        tracing::info!("Authenticating user with token: {}", &token);
        showtimes_session::verify_session(
            &state,
            &config.jwt.secret,
            showtimes_session::ShowtimesAudience::DiscordAuth,
        )
        .map_err(|err| {
            err.extend_with(|_, e| {
                e.set("reason", "invalid_state");
                e.set("state", state);
                e.set("token", token.clone());
            })
        })?;

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

                let (oauth_user, oauth_token) = showtimes_session::create_session(
                    user.id,
                    config
                        .jwt
                        .expiration
                        .unwrap_or(7 * 24 * 60 * 60)
                        .try_into()?,
                    &config.jwt.secret,
                )?;

                sess_manager
                    .lock()
                    .await
                    .set_session(&oauth_token, oauth_user)
                    .await?;

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

                let (oauth_user, oauth_token) = showtimes_session::create_session(
                    user.id,
                    config
                        .jwt
                        .expiration
                        .unwrap_or(7 * 24 * 60 * 60)
                        .try_into()?,
                    &config.jwt.secret,
                )?;

                sess_manager
                    .lock()
                    .await
                    .set_session(&oauth_token, oauth_user)
                    .await?;
                Ok(UserSessionGQL::new(user, oauth_token))
            }
        }
    }

    /// Disconnect/logout from Showtimes, this can also be used to revoke OAuth2 token
    #[graphql(guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)")]
    async fn disconnect<'a>(
        &self,
        ctx: &'a Context<'_>,
        #[graphql(desc = "Revoke specific token, this only works for Owner auth")] token: Option<
            String,
        >,
    ) -> async_graphql::Result<OkResponse> {
        let jwt = ctx.data_unchecked::<ShowtimesUserSession>();
        let sessions = ctx.data_unchecked::<SharedSessionManager>();

        match (token, jwt.get_claims().get_audience()) {
            (_, showtimes_session::ShowtimesAudience::User) => {
                sessions
                    .lock()
                    .await
                    .remove_session(jwt.get_token())
                    .await?;

                Ok(OkResponse::ok("Successfully logged out"))
            }
            (Some(token), showtimes_session::ShowtimesAudience::MasterKey) => {
                sessions.lock().await.remove_session(&token).await?;

                Ok(OkResponse::ok("Successfully revoked token"))
            }
            _ => {
                // Just stub for now
                Ok(OkResponse::ok("Successfully disconnected"))
            }
        }
    }

    /// Create a new server in Showtimes
    #[graphql(
        name = "createServer",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)"
    )]
    async fn create_server(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The server information to be created")]
        input: mutations::servers::ServerCreateInputGQL,
    ) -> async_graphql::Result<ServerGQL> {
        let user = find_authenticated_user(ctx).await?;

        mutations::servers::mutate_servers_create(ctx, user, input).await
    }

    /// Create a new project in Showtimes
    #[graphql(
        name = "createProject",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)"
    )]
    async fn create_project(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The server ID for the project")] id: crate::models::prelude::UlidGQL,
        #[graphql(desc = "The project information to be created")]
        input: mutations::projects::ProjectCreateInputGQL,
    ) -> async_graphql::Result<ProjectGQL> {
        let user = find_authenticated_user(ctx).await?;

        mutations::projects::mutate_projects_create(ctx, user, id, input).await
    }

    /// Update user information
    #[graphql(
        name = "updateUser",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)"
    )]
    async fn update_user(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The user ID to update, when NOT provided will use the current user")] id: Option<crate::models::prelude::UlidGQL>,
        #[graphql(desc = "The user information to update")] input: mutations::users::UserInputGQL,
    ) -> async_graphql::Result<UserGQL> {
        let user = find_authenticated_user(ctx).await?;
        let requested = mutations::users::UserRequester::new(user);
        let requested = if let Some(id) = id {
            requested.with_id(*id)
        } else {
            requested
        };

        mutations::users::mutate_users_update(ctx, requested, input).await
    }

    /// Update server information
    #[graphql(
        name = "updateServer",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)"
    )]
    async fn update_server(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The server ID to update")] id: crate::models::prelude::UlidGQL,
        #[graphql(desc = "The server information to update")]
        input: mutations::servers::ServerUpdateInputGQL,
    ) -> async_graphql::Result<ServerGQL> {
        let user = find_authenticated_user(ctx).await?;

        mutations::servers::mutate_servers_update(ctx, id, user, input).await
    }

    /// Update project information
    #[graphql(
        name = "updateProject",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)"
    )]
    async fn update_project(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The project ID to update")] id: crate::models::prelude::UlidGQL,
        #[graphql(desc = "The project information to update")]
        input: mutations::projects::ProjectUpdateInputGQL,
    ) -> async_graphql::Result<ProjectGQL> {
        let user = find_authenticated_user(ctx).await?;

        mutations::projects::mutate_projects_update(ctx, user, id, input).await
    }

    /// Add new episode automatically to a project
    ///
    /// This will use the last episode as the base for the new episode
    #[graphql(
        name = "projectProgressAddAuto",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)"
    )]
    async fn update_project_progress_auto_add(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The project ID to update")] id: crate::models::prelude::UlidGQL,
        #[graphql(
            desc = "The total number of episodes to add, minimum of 1 and maximum of 100",
            validator(minimum = 1, maximum = 100)
        )]
        total: u32,
    ) -> async_graphql::Result<ProjectGQL> {
        let user = find_authenticated_user(ctx).await?;

        mutations::projects::mutate_projects_episode_add_auto(ctx, user, id, total.into()).await
    }

    /// Delete a project from Showtimes
    #[graphql(
        name = "deleteProject",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)"
    )]
    async fn delete_project(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The project ID to delete")] id: crate::models::prelude::UlidGQL,
    ) -> async_graphql::Result<OkResponse> {
        let user = find_authenticated_user(ctx).await?;

        mutations::projects::mutate_projects_delete(ctx, user, id).await
    }
}

/// Create the GraphQL schema
pub fn create_schema(db_pool: &DatabaseShared) -> ShowtimesGQLSchema {
    async_graphql::Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .extension(Tracing)
        .data(DataLoader::new(
            data_loader::UserDataLoader::new(db_pool),
            tokio::spawn,
        ))
        .data(DataLoader::new(
            data_loader::ProjectDataLoader::new(db_pool),
            tokio::spawn,
        ))
        .data(DataLoader::new(
            data_loader::ServerDataLoader::new(db_pool),
            tokio::spawn,
        ))
        .data(DataLoader::new(
            data_loader::ServerSyncLoader::new(db_pool),
            tokio::spawn,
        ))
        .finish()
}
