use super::{prelude::*, projects::ProjectGQL, users::UserGQL};
use async_graphql::{Enum, Object};
use futures::TryStreamExt;
use showtimes_db::{m::ServerUser, mongodb::bson::doc, DatabaseShared};

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
}

#[Object]
impl ServerUserGQL {
    /// The complete user information
    async fn user(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<UserGQL> {
        let db = ctx.data_unchecked::<DatabaseShared>();
        let handler = showtimes_db::UserHandler::new(db);

        let user = handler.find_by_id(&self.id.to_string()).await?;

        match user {
            Some(user) => {
                let user: UserGQL = user.into();
                Ok(user.with_disable_server_fetch())
            }
            None => Err("User not found".into()),
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

impl From<ServerUser> for ServerUserGQL {
    fn from(user: ServerUser) -> Self {
        ServerUserGQL {
            id: user.id,
            privilege: user.privilege,
            extras: user.extras,
        }
    }
}

impl From<&ServerUser> for ServerUserGQL {
    fn from(user: &ServerUser) -> Self {
        ServerUserGQL {
            id: user.id,
            privilege: user.privilege,
            extras: user.extras.clone(),
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
        self.owners.iter().map(|o| o.into()).collect()
    }

    /// The server's avatar
    async fn avatar(&self) -> Option<ImageMetadataGQL> {
        self.avatar.clone().map(|a| a.into())
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
        #[graphql(
            desc = "The number of projects to return, default to 20",
            name = "perPage",
            validator(minimum = 2, maximum = 100)
        )]
        per_page: Option<u32>,
        #[graphql(desc = "The cursor to start from")] cursor: Option<UlidGQL>,
    ) -> async_graphql::Result<PaginatedGQL<ProjectGQL>> {
        if self.disable_projects {
            return Err("Projects fetch from this context is disabled to avoid looping".into());
        }

        let db = ctx.data_unchecked::<DatabaseShared>();

        // Allowed range of per_page is 10-100, with
        let per_page = per_page.filter(|&p| (2..=100).contains(&p)).unwrap_or(20);

        let project_handler = showtimes_db::ProjectHandler::new(db);
        let limit_projects = self.get_project_limits();

        if let Some(limit_proj) = &limit_projects {
            if limit_proj.is_empty() {
                let pg_info = PageInfoGQL::empty(per_page);
                return Ok(PaginatedGQL::new(Vec::new(), pg_info));
            }
        }

        let doc_query = match (cursor, limit_projects) {
            (Some(cursor), Some(limit_proj)) => {
                doc! {
                    "creator": self.id.to_string(),
                    "id": { "$gte": cursor.to_string(), "$in": limit_proj }
                }
            }
            (Some(cursor), None) => doc! {
                "creator": self.id.to_string(),
                "id": { "$gte": cursor.to_string() }
            },
            (None, Some(limitproj)) => doc! {
                "creator": self.id.to_string(),
                "id": { "$in": limitproj }
            },
            (None, None) => doc! { "creator": self.id.to_string() },
        };

        let cursor = project_handler
            .get_collection()
            .find(doc_query)
            .limit((per_page + 1) as i64)
            .sort(doc! { "id": 1 })
            .await?;
        let count = project_handler
            .get_collection()
            .count_documents(doc! { "creator": self.id.to_string() })
            .await?;

        let mut all_projects: Vec<showtimes_db::m::Project> = cursor.try_collect().await?;

        // If all_projects is equal to per_page, then there is a next page
        let last_proj = if all_projects.len() == per_page as usize {
            Some(all_projects.pop().unwrap())
        } else {
            None
        };

        let page_info = PageInfoGQL::new(count, per_page, last_proj.map(|p| p.id.into()));

        Ok(PaginatedGQL::new(
            all_projects.into_iter().map(|p| p.into()).collect(),
            page_info,
        ))
    }
}

impl From<showtimes_db::m::Server> for ServerGQL {
    fn from(server: showtimes_db::m::Server) -> Self {
        ServerGQL {
            id: server.id,
            name: server.name,
            owners: server.owners,
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
