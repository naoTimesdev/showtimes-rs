use crate::data_loader::UserDataLoader;

use super::{prelude::*, projects::ProjectGQL, users::UserGQL};
use async_graphql::{dataloader::DataLoader, Enum, ErrorExtensions, Object};
use showtimes_db::{m::ServerUser, mongodb::bson::doc};

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
        let user = loader.load_one(self.id).await?;

        match user {
            Some(user) => {
                let user: UserGQL = user.into();
                Ok(user.with_disable_server_fetch())
            }
            None => Err(
                async_graphql::Error::new(format!("User {} not found", self.id)).extend_with(
                    |_, e| {
                        e.set("reason", "not_found");
                        e.set("id", self.id.to_string());
                        e.set("server_id", self.top_server.to_string());
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
    fn from_shared(user: &ServerUser, top_server: showtimes_shared::ulid::Ulid) -> Self {
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
        #[graphql(desc = "Specify project IDs to query")] ids: Option<
            Vec<crate::models::prelude::UlidGQL>,
        >,
        #[graphql(
            name = "perPage",
            desc = "The number of projects to return, default to 20",
            validator(minimum = 2, maximum = 100)
        )]
        per_page: Option<u32>,
        #[graphql(desc = "The cursor to start from")] cursor: Option<
            crate::models::prelude::UlidGQL,
        >,
    ) -> async_graphql::Result<PaginatedGQL<ProjectGQL>> {
        if self.disable_projects {
            return Err("Projects fetch from this context is disabled to avoid looping".into());
        }

        let limit_projects = self.get_project_limits();
        let mut queries =
            crate::queries::projects::ProjectQuery::new().with_creators(vec![self.id]);
        match ids {
            Some(ids) => {
                // limit the projects to the ones that the user is managing
                let mapped_ids: Vec<showtimes_shared::ulid::Ulid> =
                    ids.into_iter().map(|id| *id).collect();
                // Filter out the projects that are not in the limit_projects
                let filtered_ids: Vec<showtimes_shared::ulid::Ulid> =
                    if let Some(limit_proj) = &limit_projects {
                        if limit_proj.is_empty() {
                            // Return an empty list if the limit_proj is empty
                            let pg_info = PageInfoGQL::empty(per_page.unwrap_or(20));
                            return Ok(PaginatedGQL::new(Vec::new(), pg_info));
                        }

                        let limit_proj: Vec<showtimes_shared::ulid::Ulid> =
                            limit_proj.iter().map(|id| id.parse().unwrap()).collect();
                        mapped_ids
                            .into_iter()
                            .filter(|&id| limit_proj.contains(&id))
                            .collect()
                    } else {
                        mapped_ids
                    };

                queries.set_ids(filtered_ids);
            }
            None => {
                if let Some(limit_proj) = &limit_projects {
                    if limit_proj.is_empty() {
                        let pg_info = PageInfoGQL::empty(per_page.unwrap_or(20));
                        return Ok(PaginatedGQL::new(Vec::new(), pg_info));
                    }

                    queries.set_ids(limit_proj.iter().map(|id| id.parse().unwrap()).collect());
                }
            }
        }
        if let Some(per_page) = per_page {
            queries.set_per_page(per_page);
        }
        if let Some(cursor) = cursor {
            queries.set_cursor(*cursor);
        }

        let results = crate::queries::projects::query_projects_paginated(ctx, queries).await?;

        Ok(results)
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
    fn get_project_limits(&self) -> Option<Vec<String>> {
        if let Some(user_id) = self.current_user {
            let user_id = user_id.to_string();
            let user = self.owners.iter().find(|u| u.id.to_string() == user_id);
            if let Some(user) = user {
                if user.privilege == showtimes_db::m::UserPrivilege::ProjectManager {
                    Some(user.extras.clone())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn with_current_user(mut self, user_id: showtimes_shared::ulid::Ulid) -> Self {
        self.current_user = Some(user_id);
        self
    }

    pub fn with_projects_disabled(mut self) -> Self {
        self.disable_projects = true;
        self
    }
}
