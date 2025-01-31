#![warn(missing_docs, clippy::empty_docs, rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../../README.md")]

use async_graphql::{dataloader::DataLoader, Context, Object};

use showtimes_db::m::APIKeyCapability;
use showtimes_gql_common::{
    data_loader::{ServerAndOwnerId, ServerDataLoader, ServerOwnerId},
    errors::GQLError,
    guard::{visible_minimum_admin, APIKeyVerify, AuthUserAndAPIKeyGuard, AuthUserMinimumGuard},
    GQLErrorCode, UserKindGQL,
};
use showtimes_gql_events::QueryEventsRoot;
use showtimes_gql_models::{
    common::PaginatedGQL,
    projects::ProjectGQL,
    search::QuerySearchRoot,
    servers::ServerGQL,
    stats::StatsGQL,
    users::{UserGQL, UserSessionGQL},
};
use showtimes_session::{ShowtimesRefreshSession, ShowtimesUserSession};

/// The main Query Root type for the GraphQL schema. This is where all the queries are defined.
pub struct QueryRoot;

/// The main Query Root type for the GraphQL schema. This is where all the queries are defined.
#[Object]
impl QueryRoot {
    /// Get current authenticated user
    #[graphql(guard = "AuthUserMinimumGuard::new(UserKindGQL::User)")]
    async fn current(&self, ctx: &Context<'_>) -> async_graphql::Result<UserSessionGQL> {
        let user_session = ctx.data_unchecked::<ShowtimesUserSession>();
        let user = ctx.data_unchecked::<showtimes_db::m::User>();

        match ctx.data_opt::<ShowtimesRefreshSession>() {
            Some(refresh_session) => Ok(UserSessionGQL::new(user, user_session.get_token())
                .with_refresh_token(refresh_session.get_token())),
            None => Ok(UserSessionGQL::new(user, user_session.get_token())),
        }
    }

    /// Get authenticated user associated servers
    #[graphql(
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::QueryServers))"
    )]
    async fn servers(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Specify server IDs to query")] ids: Option<
            Vec<showtimes_gql_common::UlidGQL>,
        >,
        #[graphql(
            name = "perPage",
            desc = "The number of servers to return, default to 20",
            validator(minimum = 2, maximum = 100)
        )]
        per_page: Option<u32>,
        #[graphql(desc = "The cursor to start from")] cursor: Option<showtimes_gql_common::UlidGQL>,
        #[graphql(desc = "Sort order, default to ID_ASC")] sort: Option<
            showtimes_gql_common::SortOrderGQL,
        >,
    ) -> async_graphql::Result<PaginatedGQL<ServerGQL>> {
        let user = ctx.data_unchecked::<showtimes_db::m::User>();

        let mut queries = showtimes_gql_paginator::servers::ServerQuery::new()
            .with_current_user(showtimes_gql_common::queries::ServerQueryUser::from(user));
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

        let results =
            showtimes_gql_paginator::servers::query_servers_paginated(ctx, queries).await?;

        let mapped_nodes: Vec<ServerGQL> = results
            .nodes()
            .iter()
            .map(|s| {
                let s_gql = ServerGQL::from(s);

                s_gql.with_current_user(user.id)
            })
            .collect();

        Ok(PaginatedGQL::new(mapped_nodes, *results.page_info()))
    }

    /// Get authenticated user associated projects
    #[graphql(
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::QueryProjects))"
    )]
    #[allow(clippy::too_many_arguments)]
    async fn projects(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Specify project IDs to query")] ids: Option<
            Vec<showtimes_gql_common::UlidGQL>,
        >,
        #[graphql(name = "serverIds", desc = "Limit projects to specific server IDs")]
        server_ids: Option<Vec<showtimes_gql_common::UlidGQL>>,
        #[graphql(
            name = "perPage",
            desc = "The number of project to return, default to 20",
            validator(minimum = 2, maximum = 100)
        )]
        per_page: Option<u32>,
        #[graphql(desc = "The cursor to start from")] cursor: Option<showtimes_gql_common::UlidGQL>,
        #[graphql(desc = "Sort order, default to ID_ASC")] sort: Option<
            showtimes_gql_common::SortOrderGQL,
        >,
        #[graphql(desc = "Remove pagination limit, this only works if you're an Admin")]
        unpaged: bool,
    ) -> async_graphql::Result<PaginatedGQL<ProjectGQL>> {
        let user = ctx.data_unchecked::<showtimes_db::m::User>();

        let allowed_servers = match user.kind {
            showtimes_db::m::UserKind::User => {
                let projector = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();

                projector.load_one(ServerOwnerId::new(user.id)).await?
            }
            _ => None,
        };

        let mut queries = showtimes_gql_paginator::projects::ProjectQuery::new()
            .with_current_user(user.clone().into());
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

        let results =
            showtimes_gql_paginator::projects::query_projects_paginated(ctx, queries).await?;

        let mapped_nodes: Vec<ProjectGQL> = results.nodes().iter().map(ProjectGQL::from).collect();

        Ok(PaginatedGQL::new(mapped_nodes, *results.page_info()))
    }

    /// Get all available users, you need a minimum of admin role to access this
    #[graphql(
        guard = "AuthUserMinimumGuard::new(UserKindGQL::Admin)",
        visible = "visible_minimum_admin"
    )]
    async fn users(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Specify user IDs to query")] ids: Option<
            Vec<showtimes_gql_common::UlidGQL>,
        >,
        #[graphql(
            name = "perPage",
            desc = "The number of users to return, default to 20",
            validator(minimum = 2, maximum = 100)
        )]
        per_page: Option<u32>,
        #[graphql(desc = "The cursor to start from")] cursor: Option<showtimes_gql_common::UlidGQL>,
        #[graphql(desc = "Sort order, default to ID_ASC")] sort: Option<
            showtimes_gql_common::SortOrderGQL,
        >,
    ) -> async_graphql::Result<PaginatedGQL<UserGQL>> {
        let user = ctx.data_unchecked::<showtimes_db::m::User>();
        let mut queries = showtimes_gql_paginator::users::UserQuery::new()
            .with_current_user(showtimes_gql_common::queries::ServerQueryUser::from(user));
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

        let results = showtimes_gql_paginator::users::query_users_paginated(ctx, queries).await?;

        let mapped_nodes: Vec<UserGQL> = results.nodes().iter().map(UserGQL::from).collect();

        Ok(PaginatedGQL::new(mapped_nodes, *results.page_info()))
    }

    /// Get server statistics
    #[graphql(
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::QueryStats))"
    )]
    async fn stats(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Specify server ID to query")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<StatsGQL> {
        let user = ctx.data_unchecked::<showtimes_db::m::User>();
        let projector = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();
        let server = match user.kind {
            showtimes_db::m::UserKind::User => {
                projector
                    .load_one(ServerAndOwnerId::new(*id, user.id))
                    .await?
            }
            _ => projector.load_one(*id).await?,
        }
        .ok_or_else(|| {
            GQLError::new("Server not found", GQLErrorCode::ServerNotFound)
                .extend(|e| e.set("id", id.to_string()))
        })?;

        Ok(StatsGQL::new(server))
    }

    /// Do a external searvice metadata search
    #[graphql(
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::QuerySearch))"
    )]
    async fn search(&self) -> QuerySearchRoot {
        // This is just an empty root which has dynamic fields
        QuerySearchRoot
    }

    /// Query events updates from specific IDs
    ///
    /// Warning: This branch of query will return all updates from your provided IDs. It's recommended
    /// to use the equivalent subscription instead for real-time updates. This is mainly used to get
    /// older updates that is not yet processed by the client connecting to the subscription.
    #[graphql(
        guard = "AuthUserMinimumGuard::new(UserKindGQL::Admin)",
        visible = "visible_minimum_admin"
    )]
    async fn events(&self) -> QueryEventsRoot {
        // This is just an empty root which has dynamic fields
        QueryEventsRoot
    }
}
