#![doc = include_str!("../README.md")]

use std::sync::LazyLock;

use futures::{Stream, StreamExt};

use async_graphql::dataloader::DataLoader;
use async_graphql::extensions::Tracing;
use async_graphql::{Context, Object, Subscription};
use data_loader::{find_authenticated_user, ServerAndOwnerId, ServerDataLoader, ServerOwnerId};
use models::collaborations::{CollaborationInviteGQL, CollaborationSyncGQL};
use models::events::prelude::EventGQL;
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
use showtimes_session::ShowtimesUserSession;

mod data_loader;
mod guard;
mod image;
mod models;
mod mutations;
mod queries;

pub type ShowtimesGQLSchema = async_graphql::Schema<QueryRoot, MutationRoot, SubscriptionRoot>;
pub use async_graphql::http::{graphiql_plugin_explorer, GraphiQLSource, ALL_WEBSOCKET_PROTOCOLS};
pub use async_graphql::{Data, Error};
pub use image::MAX_IMAGE_SIZE;

static STUBBED_ADMIN: LazyLock<ServerQueryUser> = LazyLock::new(|| {
    ServerQueryUser::new(
        showtimes_shared::ulid::Ulid::new(),
        showtimes_db::m::UserKind::Admin,
    )
});

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

        mutations::projects::mutate_projects_episode_add_manual(ctx, user, id, &episodes).await
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

        mutations::projects::mutate_projects_episode_remove(ctx, user, id, &episodes).await
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

        mutations::collaborations::mutate_collaborations_initiate(ctx, user, input).await
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

        mutations::collaborations::mutate_collaborations_accept(ctx, user, id).await
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

        mutations::collaborations::mutate_collaborations_cancel(ctx, user, id, true).await
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

        mutations::collaborations::mutate_collaborations_cancel(ctx, user, id, false).await
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
}

pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Watch for user created events
    ///
    /// Because of limitation in async-graphql, we sadly cannot combine stream of
    /// our broker with the stream from ClickHouse data if user provided a start IDs.
    #[graphql(
        name = "watchUserCreated",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_user_created(&self) -> impl Stream<Item = EventGQL<UserCreatedEventDataGQL>> {
        // TODO: Find a way to combine this with ClickHouse data
        showtimes_events::MemoryBroker::<showtimes_events::m::UserCreatedEvent>::subscribe().map(
            move |event| {
                let inner = UserCreatedEventDataGQL::new(event.data(), *STUBBED_ADMIN);
                EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                )
            },
        )
    }

    /// Watch for user updates events
    ///
    /// Because of limitation in async-graphql, we sadly cannot combine stream of
    /// our broker with the stream from ClickHouse data if user provided a start IDs.
    #[graphql(
        name = "watchUserUpdated",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_user_updated(&self) -> impl Stream<Item = EventGQL<UserUpdatedEventDataGQL>> {
        // TODO: Find a way to combine this with ClickHouse data
        showtimes_events::MemoryBroker::<showtimes_events::m::UserUpdatedEvent>::subscribe().map(
            move |event| {
                let inner = UserUpdatedEventDataGQL::new(event.data(), *STUBBED_ADMIN);
                EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                )
            },
        )
    }

    /// Watch for user deleted events
    ///
    /// Because of limitation in async-graphql, we sadly cannot combine stream of
    /// our broker with the stream from ClickHouse data if user provided a start IDs.
    #[graphql(
        name = "watchUserDeleted",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_user_deleted(&self) -> impl Stream<Item = EventGQL<UserDeletedEventDataGQL>> {
        // TODO: Find a way to combine this with ClickHouse data
        showtimes_events::MemoryBroker::<showtimes_events::m::UserDeletedEvent>::subscribe().map(
            move |event| {
                let inner = UserDeletedEventDataGQL::new(event.data());
                EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                )
            },
        )
    }

    /// Watch for server created events
    ///
    /// Because of limitation in async-graphql, we sadly cannot combine stream of
    /// our broker with the stream from ClickHouse data if user provided a start IDs.
    #[graphql(
        name = "watchServerCreated",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_server_created(
        &self,
    ) -> impl Stream<Item = EventGQL<ServerCreatedEventDataGQL>> {
        // TODO: Find a way to combine this with ClickHouse data
        showtimes_events::MemoryBroker::<showtimes_events::m::ServerCreatedEvent>::subscribe().map(
            move |event| {
                let inner = ServerCreatedEventDataGQL::new(event.data().id());
                EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                )
            },
        )
    }

    /// Watch for server updates events
    ///
    /// Because of limitation in async-graphql, we sadly cannot combine stream of
    /// our broker with the stream from ClickHouse data if user provided a start IDs.
    #[graphql(
        name = "watchServerUpdated",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_server_updated(
        &self,
    ) -> impl Stream<Item = EventGQL<ServerUpdatedEventDataGQL>> {
        // TODO: Find a way to combine this with ClickHouse data
        showtimes_events::MemoryBroker::<showtimes_events::m::ServerUpdatedEvent>::subscribe().map(
            move |event| {
                let inner = ServerUpdatedEventDataGQL::from(event.data());
                EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                )
            },
        )
    }

    /// Watch for server deleted events
    ///
    /// Because of limitation in async-graphql, we sadly cannot combine stream of
    /// our broker with the stream from ClickHouse data if user provided a start IDs.
    #[graphql(
        name = "watchServerDeleted",
        guard = "guard::AuthUserMinimumGuard::new(models::users::UserKindGQL::Admin)"
    )]
    async fn watch_server_deleted(
        &self,
    ) -> impl Stream<Item = EventGQL<ServerDeletedEventDataGQL>> {
        // TODO: Find a way to combine this with ClickHouse data
        showtimes_events::MemoryBroker::<showtimes_events::m::ServerDeletedEvent>::subscribe().map(
            move |event| {
                let inner = ServerDeletedEventDataGQL::from(event.data());
                EventGQL::new(
                    event.id(),
                    inner,
                    event.kind().into(),
                    event.actor().map(|a| a.to_string()),
                    event.timestamp(),
                )
            },
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
