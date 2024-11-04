#![warn(missing_docs, clippy::empty_docs, rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../README.md")]

use std::sync::{Arc, LazyLock};

use futures_util::{Stream, StreamExt};

use async_graphql::dataloader::DataLoader;
use async_graphql::extensions::Tracing;
use async_graphql::{Context, Object, Subscription};
use data_loader::{
    find_authenticated_user, ServerAndOwnerId, ServerDataLoader, ServerOwnerId, UserDataLoader,
};
use models::collaborations::{CollaborationInviteGQL, CollaborationSyncGQL};
use models::events::collaborations::{
    CollabAcceptedEventDataGQL, CollabCreatedEventDataGQL, CollabDeletedEventDataGQL,
    CollabRejectedEventDataGQL, CollabRetractedEventDataGQL,
};
use models::events::prelude::EventGQL;
use models::events::projects::{
    ProjectCreatedEventDataGQL, ProjectDeletedEventDataGQL, ProjectEpisodeUpdatedEventDataGQL,
    ProjectUpdatedEventDataGQL,
};
use models::events::servers::{
    ServerCreatedEventDataGQL, ServerDeletedEventDataGQL, ServerUpdatedEventDataGQL,
};
use models::events::users::{
    UserCreatedEventDataGQL, UserDeletedEventDataGQL, UserUpdatedEventDataGQL,
};
use models::events::QueryEventsRoot;
use models::prelude::{OkResponse, PaginatedGQL};
use models::projects::ProjectGQL;
use models::search::QuerySearchRoot;
use models::servers::ServerGQL;
use models::stats::StatsGQL;
use models::users::{UserGQL, UserSessionGQL};
use queries::ServerQueryUser;
use showtimes_db::{mongodb::bson::doc, DatabaseShared};
use showtimes_session::manager::SharedSessionManager;
use showtimes_session::{ShowtimesRefreshSession, ShowtimesUserSession};

mod data_loader;
mod expand;
mod guard;
mod image;
mod models;
mod mutations;
mod queries;

/// The main schema for our GraphQL server.
///
/// Wraps [`QueryRoot`], [`MutationRoot`], and [`SubscriptionRoot`] types.
pub type ShowtimesGQLSchema = async_graphql::Schema<QueryRoot, MutationRoot, SubscriptionRoot>;
pub use async_graphql::http::{graphiql_plugin_explorer, GraphiQLSource, ALL_WEBSOCKET_PROTOCOLS};
pub use async_graphql::{Data, Error};
pub(crate) use expand::{
    expand_combined_stream_event, expand_query_event, expand_query_event_with_user,
};
pub use image::MAX_IMAGE_SIZE;
pub use models::Orchestrator;

static STUBBED_ADMIN: LazyLock<ServerQueryUser> = LazyLock::new(|| {
    ServerQueryUser::new(
        showtimes_shared::ulid::Ulid::new(),
        showtimes_db::m::UserKind::Admin,
    )
});

/// The main Query Root type for the GraphQL schema. This is where all the queries are defined.
pub struct QueryRoot;

/// The main Query Root type for the GraphQL schema. This is where all the queries are defined.
#[Object]
impl QueryRoot {
    /// Get current authenticated user
    #[graphql(guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)")]
    async fn current<'a>(&self, ctx: &'a Context<'_>) -> async_graphql::Result<UserSessionGQL> {
        let user_session = ctx.data_unchecked::<ShowtimesUserSession>();
        let user = find_authenticated_user(ctx).await?;

        match ctx.data_opt::<ShowtimesRefreshSession>() {
            Some(refresh_session) => Ok(UserSessionGQL::new(user, user_session.get_token())
                .with_refresh_token(refresh_session.get_token())),
            None => Ok(UserSessionGQL::new(user, user_session.get_token())),
        }
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

    /// Query events updates from specific IDs
    ///
    /// Warning: This branch of query will return all updates from your provided IDs. It's recommended
    /// to use the equivalent subscription instead for real-time updates. This is mainly used to get
    /// older updates that is not yet processed by the client connecting to the subscription.
    #[graphql(guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)")]
    async fn events(&self) -> QueryEventsRoot {
        // This is just an empty root which has dynamic fields
        QueryEventsRoot
    }
}

/// The main Mutation Root type for the GraphQL schema. This is where all the mutation are defined.
pub struct MutationRoot;

/// The main Mutation Root type for the GraphQL schema. This is where all the mutation are defined.
#[Object]
impl MutationRoot {
    /// Authorize Discord OAuth2 token and state that was returned from the OAuth2 redirect
    async fn auth<'a>(
        &self,
        ctx: &'a Context<'_>,
        #[graphql(desc = "The OAuth2 token/code returned from Discord")] token: String,
        #[graphql(desc = "The OAuth2 state")] state: String,
    ) -> async_graphql::Result<UserSessionGQL> {
        crate::mutations::users::mutate_users_authenticate(ctx, token, state).await
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
        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        mutations::projects::mutate_projects_update(ctx, user_behalf.unwrap_or(user), id, input)
            .await
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
        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        mutations::projects::mutate_projects_episode_add_auto(
            ctx,
            user_behalf.unwrap_or(user),
            id,
            total.into(),
        )
        .await
    }

    /// Add new episode manually to a project
    ///
    /// You will need to provide each episode information
    #[graphql(
        name = "projectProgressAdd",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)"
    )]
    async fn update_project_progress_add(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The project ID to update")] id: crate::models::prelude::UlidGQL,
        #[graphql(
            desc = "The new episodes to be added, minimum of 1 and maximum of 100",
            validator(min_items = 1, max_items = 100)
        )]
        episodes: Vec<mutations::projects::ProgressCreateInputGQL>,
    ) -> async_graphql::Result<ProjectGQL> {
        let user = find_authenticated_user(ctx).await?;
        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        mutations::projects::mutate_projects_episode_add_manual(
            ctx,
            user_behalf.unwrap_or(user),
            id,
            &episodes,
        )
        .await
    }

    /// Add new episode automatically to a project
    ///
    /// This will use the last episode as the base for the new episode
    #[graphql(
        name = "projectProgressRemove",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)"
    )]
    async fn update_project_progress_remove(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The project ID to update")] id: crate::models::prelude::UlidGQL,
        #[graphql(
            desc = "The episodes to delete, minimum of 1 and maximum of 100",
            validator(min_items = 1, max_items = 100)
        )]
        episodes: Vec<u64>,
    ) -> async_graphql::Result<ProjectGQL> {
        let user = find_authenticated_user(ctx).await?;
        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        mutations::projects::mutate_projects_episode_remove(
            ctx,
            user_behalf.unwrap_or(user),
            id,
            &episodes,
        )
        .await
    }

    /// Initiate a collaboration between projects
    #[graphql(
        name = "collaborateProjects",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)"
    )]
    async fn collaborate_projects(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The collaboration information to be created")]
        input: mutations::collaborations::CollaborationRequestInputGQL,
    ) -> async_graphql::Result<CollaborationInviteGQL> {
        let user = find_authenticated_user(ctx).await?;
        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        mutations::collaborations::mutate_collaborations_initiate(
            ctx,
            user_behalf.unwrap_or(user),
            input,
        )
        .await
    }

    /// Accept a collaboration request between projects
    #[graphql(
        name = "collaborateAccept",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)"
    )]
    async fn collaborate_accept(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The collaboration ID to accept")] id: crate::models::prelude::UlidGQL,
    ) -> async_graphql::Result<CollaborationSyncGQL> {
        let user = find_authenticated_user(ctx).await?;
        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        mutations::collaborations::mutate_collaborations_accept(
            ctx,
            user_behalf.unwrap_or(user),
            id,
        )
        .await
    }

    /// Deny a collaboration request between projects
    #[graphql(
        name = "collaborateDeny",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)"
    )]
    async fn collaborate_deny(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The collaboration ID to deny")] id: crate::models::prelude::UlidGQL,
    ) -> async_graphql::Result<OkResponse> {
        let user = find_authenticated_user(ctx).await?;
        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        mutations::collaborations::mutate_collaborations_cancel(
            ctx,
            user_behalf.unwrap_or(user),
            id,
            true,
        )
        .await
    }

    /// Retract a collaboration request between projects
    #[graphql(
        name = "collaborateRetract",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)"
    )]
    async fn collaborate_retract(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The collaboration ID to retract")] id: crate::models::prelude::UlidGQL,
    ) -> async_graphql::Result<OkResponse> {
        let user = find_authenticated_user(ctx).await?;
        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        mutations::collaborations::mutate_collaborations_cancel(
            ctx,
            user_behalf.unwrap_or(user),
            id,
            false,
        )
        .await
    }

    /// Delete or unlink a collaboration between projects
    #[graphql(
        name = "collaborateDelete",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)"
    )]
    async fn collaborate_delete(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The collaboration ID target")] id: crate::models::prelude::UlidGQL,
        #[graphql(desc = "The target project to delete or remove")]
        target: crate::models::prelude::UlidGQL,
    ) -> async_graphql::Result<OkResponse> {
        let user = find_authenticated_user(ctx).await?;
        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        mutations::collaborations::mutate_collaborations_unlink(
            ctx,
            user_behalf.unwrap_or(user),
            id,
            target,
        )
        .await
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
        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        mutations::projects::mutate_projects_delete(ctx, user_behalf.unwrap_or(user), id).await
    }

    /// Delete a server from Showtimes
    #[graphql(
        name = "deleteServer",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::User)"
    )]
    async fn delete_server(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The server ID to delete")] id: crate::models::prelude::UlidGQL,
    ) -> async_graphql::Result<OkResponse> {
        let user = find_authenticated_user(ctx).await?;

        mutations::servers::mutate_servers_delete(ctx, user, id).await
    }

    /// Create a session for another user.
    ///
    /// This is mainly used by other services to orchestrate Showtimes on behalf of the user.
    ///
    /// Only available for Admin users.
    #[graphql(
        name = "createSession",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn create_session(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The user ID to create session for")] id: crate::models::prelude::UlidGQL,
    ) -> async_graphql::Result<UserSessionGQL> {
        let loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();
        let session = ctx.data_unchecked::<SharedSessionManager>();
        let config = ctx.data_unchecked::<Arc<showtimes_shared::config::Config>>();
        let user = loader.load_one(*id).await?;

        match user {
            Some(user) => {
                // Create actual session
                let (claims, _) = showtimes_session::create_session(
                    user.id,
                    config.jwt.get_expiration() as i64,
                    &config.jwt.secret,
                )?;

                // We don't create refresh token session for this custom orchestration.
                let mut sess_mutex = session.lock().await;
                sess_mutex
                    .set_session(claims.get_token(), claims.get_claims())
                    .await?;
                drop(sess_mutex);

                Ok(UserSessionGQL::new(user, claims.get_token()))
            }
            None => Err("User not found".into()),
        }
    }
}

/// The main Subscription Root type for the GraphQL schema. This is where all the subscription are defined.
pub struct SubscriptionRoot;

/// The main Subscription Root type for the GraphQL schema. This is where all the subscription are defined.
#[Subscription]
impl SubscriptionRoot {
    /// Watch for user created events
    #[graphql(
        name = "watchUserCreated",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_user_created(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<models::prelude::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<UserCreatedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::UserCreated,
            showtimes_events::m::UserCreatedEvent,
            UserCreatedEventDataGQL,
            *STUBBED_ADMIN
        )
    }

    /// Watch for user updates events
    #[graphql(
        name = "watchUserUpdated",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_user_updated(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<models::prelude::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<UserUpdatedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::UserUpdated,
            showtimes_events::m::UserUpdatedEvent,
            UserUpdatedEventDataGQL,
            *STUBBED_ADMIN
        )
    }

    /// Watch for user deleted events
    #[graphql(
        name = "watchUserDeleted",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_user_deleted(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<models::prelude::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<UserDeletedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::UserDeleted,
            showtimes_events::m::UserDeletedEvent,
            UserDeletedEventDataGQL
        )
    }

    /// Watch for server created events
    #[graphql(
        name = "watchServerCreated",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_server_created(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<models::prelude::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<ServerCreatedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::ServerCreated,
            showtimes_events::m::ServerCreatedEvent,
            ServerCreatedEventDataGQL
        )
    }

    /// Watch for server updates events
    #[graphql(
        name = "watchServerUpdated",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_server_updated(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<models::prelude::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<ServerUpdatedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::ServerUpdated,
            showtimes_events::m::ServerUpdatedEvent,
            ServerUpdatedEventDataGQL
        )
    }

    /// Watch for server deleted events
    #[graphql(
        name = "watchServerDeleted",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_server_deleted(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<models::prelude::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<ServerDeletedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::ServerDeleted,
            showtimes_events::m::ServerDeletedEvent,
            ServerDeletedEventDataGQL
        )
    }

    /// Watch for project created events
    #[graphql(
        name = "watchProjectCreated",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_project_created(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<models::prelude::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<ProjectCreatedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::ProjectCreated,
            showtimes_events::m::ProjectCreatedEvent,
            ProjectCreatedEventDataGQL
        )
    }

    /// Watch for project updates events
    #[graphql(
        name = "watchProjectUpdated",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_project_updated(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<models::prelude::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<ProjectUpdatedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::ProjectUpdated,
            showtimes_events::m::ProjectUpdatedEvent,
            ProjectUpdatedEventDataGQL
        )
    }

    /// Watch for project episodes update events
    #[graphql(
        name = "watchProjectEpisodeUpdated",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_project_episode_updated(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<models::prelude::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<ProjectEpisodeUpdatedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::ProjectEpisodes,
            showtimes_events::m::ProjectEpisodeUpdatedEvent,
            ProjectEpisodeUpdatedEventDataGQL
        )
    }

    /// Watch for project deleted events
    #[graphql(
        name = "watchProjectDeleted",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_project_deleted(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<models::prelude::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<ProjectDeletedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::ProjectDeleted,
            showtimes_events::m::ProjectDeletedEvent,
            ProjectDeletedEventDataGQL
        )
    }

    /// Watch for collaboration created events
    #[graphql(
        name = "watchCollabCreated",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_collab_created(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<models::prelude::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<CollabCreatedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::CollaborationCreated,
            showtimes_events::m::CollabCreatedEvent,
            CollabCreatedEventDataGQL
        )
    }

    /// Watch for collaboration acceptances events
    #[graphql(
        name = "watchCollabAccepted",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_collab_accepted(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<models::prelude::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<CollabAcceptedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::CollaborationAccepted,
            showtimes_events::m::CollabAcceptedEvent,
            CollabAcceptedEventDataGQL
        )
    }

    /// Watch for collaboration rejection events
    #[graphql(
        name = "watchCollabRejected",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_collab_rejected(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<models::prelude::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<CollabRejectedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::CollaborationRejected,
            showtimes_events::m::CollabRejectedEvent,
            CollabRejectedEventDataGQL
        )
    }

    /// Watch for collaboration retraction events
    #[graphql(
        name = "watchCollabRetracted",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_collab_retracted(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<models::prelude::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<CollabRetractedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::CollaborationRetracted,
            showtimes_events::m::CollabRetractedEvent,
            CollabRetractedEventDataGQL
        )
    }

    /// Watch for collaboration deleted events
    #[graphql(
        name = "watchCollabDeleted",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_collab_deleted(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The starting ID to query")] id: Option<models::prelude::UlidGQL>,
    ) -> impl Stream<Item = EventGQL<CollabDeletedEventDataGQL>> {
        expand_combined_stream_event!(
            ctx,
            id,
            showtimes_events::m::EventKind::CollaborationDeleted,
            showtimes_events::m::CollabDeletedEvent,
            CollabDeletedEventDataGQL
        )
    }
}

/// Create the GraphQL schema
pub fn create_schema(db_pool: &DatabaseShared) -> ShowtimesGQLSchema {
    async_graphql::Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
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
