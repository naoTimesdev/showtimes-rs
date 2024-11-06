//! A server models list

use async_graphql::{dataloader::DataLoader, Enum, ErrorExtensions, Object};
use showtimes_db::{m::ServerUser, mongodb::bson::doc};
use showtimes_gql_common::{data_loader::UserDataLoader, queries::MinimalServerUsers, *};
use showtimes_gql_paginator::projects::ProjectQuery;

use crate::common::PaginatedGQL;

use super::{projects::ProjectGQL, users::UserGQL};

/// Enum to hold user privileges on a server.
///
/// There is no "normal" user, as all users are considered normal.
#[derive(Enum, Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[graphql(remote = "showtimes_db::m::UserPrivilege")]
pub enum UserPrivilegeGQL {
    /// A project manager on a server
    ///
    /// This user can:
    /// - Manage single or multiple projects
    ProjectManager,
    /// A manager of the server
    ///
    /// In addition to project manager, this user can:
    /// - Add and remove project
    /// - Manage all project
    Manager,
    /// A user with all the special privileges
    ///
    /// In addition to manager, this user can:
    /// - Add and remove users
    /// - Manage the server settings
    Admin,
    /// A user with complete control over the server
    ///
    /// In addition to admin, this user can:
    /// - Delete the server
    /// - Add or remove admins
    ///
    /// Only one user can have this privilege
    Owner,
}

/// Owner information in the server
pub struct ServerUserGQL {
    /// The associated user ID
    id: showtimes_shared::ulid::Ulid,
    /// The user's privilege
    privilege: showtimes_db::m::UserPrivilege,
    /// The extra associated data with the user
    ///
    /// Used to store extra data like what projects the user is managing
    extras: Vec<String>,
    top_server: showtimes_shared::ulid::Ulid,
}

#[Object]
impl ServerUserGQL {
    /// The complete user information
    async fn user(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<UserGQL> {
        let loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();
        let user = loader.load_one(self.id).await.map_err(|e| {
            e.extend_with(|_, e| {
                e.set("id", self.id.to_string());
                e.set("server_id", self.top_server.to_string());
                e.set("reason", GQLError::UserRequestFails);
                e.set("code", GQLError::UserRequestFails.code());
            })
        })?;

        match user {
            Some(user) => {
                let user: UserGQL = user.into();
                Ok(user.with_disable_server_fetch())
            }
            None => Err(
                async_graphql::Error::new(format!("User {} not found", self.id)).extend_with(
                    |_, e| {
                        e.set("id", self.id.to_string());
                        e.set("server_id", self.top_server.to_string());
                        e.set("reason", GQLError::UserRequestFails);
                        e.set("code", GQLError::UserRequestFails.code());
                    },
                ),
            ),
        }
    }

    /// The user's privilege
    async fn privilege(&self) -> UserPrivilegeGQL {
        self.privilege.into()
    }

    /// The extra associated data with the user
    ///
    /// Used to store extra data like what projects the user is managing
    async fn extras(&self) -> Vec<String> {
        self.extras.clone()
    }
}

impl ServerUserGQL {
    /// Create a new server user
    pub fn from_shared(user: &ServerUser, top_server: showtimes_shared::ulid::Ulid) -> Self {
        ServerUserGQL {
            id: user.id,
            privilege: user.privilege,
            extras: user.extras.clone(),
            top_server,
        }
    }
}

/// A model to hold server information
///
/// The original account is called "server" as a caddy over from the original
/// project. This is a server in the sense of a project server, not a physical
pub struct ServerGQL {
    id: showtimes_shared::ulid::Ulid,
    name: String,
    owners: Vec<ServerUser>,
    integrations: Vec<showtimes_db::m::IntegrationId>,
    avatar: Option<showtimes_db::m::ImageMetadata>,
    created: chrono::DateTime<chrono::Utc>,
    updated: chrono::DateTime<chrono::Utc>,
    current_user: Option<showtimes_shared::ulid::Ulid>,
    disable_projects: bool,
}

#[Object]
impl ServerGQL {
    /// The server's ID
    async fn id(&self) -> UlidGQL {
        self.id.into()
    }

    /// The server's name
    async fn name(&self) -> String {
        self.name.clone()
    }

    /// The server's owners
    async fn owners(&self) -> Vec<ServerUserGQL> {
        self.owners
            .iter()
            .map(|o| ServerUserGQL::from_shared(o, self.id))
            .collect()
    }

    /// The server's avatar
    async fn avatar(&self) -> Option<ImageMetadataGQL> {
        self.avatar.clone().map(|a| a.into())
    }

    /// The server integrations information.
    ///
    /// Can be used to link to other services like Discord or FansubDB.
    async fn integrations(&self) -> Vec<IntegrationIdGQL> {
        self.integrations.iter().map(|i| i.into()).collect()
    }

    /// The server's creation date
    async fn created(&self) -> DateTimeGQL {
        self.created.into()
    }

    /// The server's last update date
    async fn updated(&self) -> DateTimeGQL {
        self.updated.into()
    }

    /// The list of server projects
    async fn projects(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Specify project IDs to query")] ids: Option<Vec<UlidGQL>>,
        #[graphql(
            name = "perPage",
            desc = "The number of projects to return, default to 20",
            validator(minimum = 2, maximum = 100)
        )]
        per_page: Option<u32>,
        #[graphql(desc = "The cursor to start from")] cursor: Option<UlidGQL>,
        #[graphql(desc = "Sort order, default to ID_ASC")] sort: Option<SortOrderGQL>,
    ) -> async_graphql::Result<PaginatedGQL<ProjectGQL>> {
        if self.disable_projects {
            return Err("Projects fetch from this context is disabled to avoid looping".into());
        }

        let mut queries = ProjectQuery::new()
            .with_creators(vec![self.id])
            .with_allowed_servers_minimal(vec![MinimalServerUsers::new(
                self.id,
                self.owners.clone(),
            )]);

        if let Some(ids) = ids {
            queries.set_ids(ids.into_iter().map(|i| *i).collect());
        }
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
            showtimes_gql_paginator::projects::query_projects_paginated(ctx, queries).await?;

        let mapped_nodes: Vec<ProjectGQL> = results.nodes().iter().map(ProjectGQL::from).collect();

        Ok(PaginatedGQL::new(mapped_nodes, *results.page_info()))
    }
}

impl From<showtimes_db::m::Server> for ServerGQL {
    fn from(server: showtimes_db::m::Server) -> Self {
        ServerGQL {
            id: server.id,
            name: server.name,
            owners: server.owners,
            integrations: server.integrations,
            avatar: server.avatar,
            created: server.created,
            updated: server.updated,
            current_user: None,
            disable_projects: false,
        }
    }
}

impl From<&showtimes_db::m::Server> for ServerGQL {
    fn from(server: &showtimes_db::m::Server) -> Self {
        ServerGQL {
            id: server.id,
            name: server.name.clone(),
            owners: server.owners.clone(),
            avatar: server.avatar.clone(),
            integrations: server.integrations.clone(),
            created: server.created,
            updated: server.updated,
            current_user: None,
            disable_projects: false,
        }
    }
}

impl ServerGQL {
    /// Add the current user to the server guard
    pub fn with_current_user(mut self, user_id: showtimes_shared::ulid::Ulid) -> Self {
        self.current_user = Some(user_id);
        self
    }

    /// Disable project fetch
    pub fn with_projects_disabled(mut self) -> Self {
        self.disable_projects = true;
        self
    }
}
