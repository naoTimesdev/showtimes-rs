//! A collaboration sync or invite models list

use async_graphql::{dataloader::DataLoader, Object};
use errors::GQLError;
use showtimes_db::m::APIKeyCapability;
use showtimes_shared::ulid::Ulid;

use showtimes_gql_common::data_loader::{ProjectDataLoader, ServerDataLoader};
use showtimes_gql_common::*;

use super::{projects::ProjectGQL, servers::ServerGQL};

/// Collaboration sync initiator
#[derive(Debug, Clone, Copy)]
pub struct CollaborationSyncRequester {
    server_id: showtimes_shared::ulid::Ulid,
    project_id: showtimes_shared::ulid::Ulid,
}

impl CollaborationSyncRequester {
    /// Create a new instance of [`CollaborationSyncRequester`]
    pub fn new(server_id: Ulid, project_id: Ulid) -> Self {
        CollaborationSyncRequester {
            server_id,
            project_id,
        }
    }
}

/// Collaboration sync information
pub struct CollaborationSyncGQL {
    /// The collaboration ID
    id: showtimes_shared::ulid::Ulid,
    /// The server ID
    server_ids: Vec<showtimes_shared::ulid::Ulid>,
    /// The project ID
    project_ids: Vec<showtimes_shared::ulid::Ulid>,
    /// Collaboration link creation
    created: chrono::DateTime<chrono::Utc>,
    /// Collaboration link updated
    updated: chrono::DateTime<chrono::Utc>,
    /// What server requested the sync
    requester: Option<CollaborationSyncRequester>,
}

impl CollaborationSyncGQL {
    /// Create a new instance of [`CollaborationSyncGQL`]
    pub fn new(
        db: &showtimes_db::m::ServerCollaborationSync,
        requester: CollaborationSyncRequester,
    ) -> Self {
        let server_ids: Vec<Ulid> = db.projects.iter().map(|p| p.server).collect();
        let project_ids: Vec<Ulid> = db.projects.iter().map(|p| p.project).collect();

        CollaborationSyncGQL {
            id: db.id,
            server_ids,
            project_ids,
            created: db.created,
            updated: db.updated,
            requester: Some(requester),
        }
    }
}

#[Object]
impl CollaborationSyncGQL {
    /// The collaboration ID
    async fn id(&self) -> UlidGQL {
        self.id.into()
    }

    /// The server information
    #[graphql(
        guard = "guard::AuthAPIKeyMinimumGuard::new(guard::APIKeyVerify::Specific(APIKeyCapability::QueryServers))"
    )]
    async fn servers(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Vec<ServerGQL>> {
        let loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();

        let unmapped_servers_ids: Vec<Ulid> = match self.requester {
            Some(requester) => self
                .server_ids
                .clone()
                .into_iter()
                .filter(|id| *id != requester.server_id)
                .collect(),
            None => self.server_ids.clone(),
        };

        if unmapped_servers_ids.is_empty() {
            return Ok(vec![]);
        }

        let results_mapped: Vec<ServerGQL> = loader
            .load_many(unmapped_servers_ids)
            .await?
            .into_values()
            .map(|srv| {
                let srv_gql: ServerGQL = srv.into();
                srv_gql.with_projects_disabled()
            })
            .collect();

        Ok(results_mapped)
    }

    /// All the attached projects
    #[graphql(
        guard = "guard::AuthAPIKeyMinimumGuard::new(guard::APIKeyVerify::Specific(APIKeyCapability::QueryProjects))"
    )]
    async fn projects(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Vec<ProjectGQL>> {
        let loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();

        let unmapped_project_ids: Vec<showtimes_shared::ulid::Ulid> = self
            .project_ids
            .clone()
            .into_iter()
            .filter(|id| match &self.requester {
                Some(requester) => id != &requester.project_id,
                None => true,
            })
            .collect();

        if unmapped_project_ids.is_empty() {
            return Ok(vec![]);
        }

        let results_mapped: Vec<ProjectGQL> = loader
            .load_many(unmapped_project_ids)
            .await?
            .into_values()
            .map(|proj| {
                let prj_gql: ProjectGQL = proj.into();
                prj_gql
                    .with_disable_server_fetch()
                    .with_disable_collaboration_fetch()
            })
            .collect();

        Ok(results_mapped)
    }

    /// The collaboration link creation
    async fn created(&self) -> DateTimeGQL {
        self.created.into()
    }

    /// The collaboration link updated
    async fn updated(&self) -> DateTimeGQL {
        self.updated.into()
    }
}

/// The collaboration invite source data
pub struct CollaborationInviteSourceGQL {
    /// The server ID
    server_id: showtimes_shared::ulid::Ulid,
    /// The project ID
    project_id: showtimes_shared::ulid::Ulid,
}

#[Object]
impl CollaborationInviteSourceGQL {
    /// The server information
    async fn server(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<ServerGQL> {
        let loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();

        let srv = loader.load_one(self.server_id).await?.ok_or_else(|| {
            GQLError::new("Server not found", GQLErrorCode::ServerNotFound)
                .extend(|e| e.set("id", self.server_id.to_string()))
        })?;

        let srv_gql: ServerGQL = srv.into();
        Ok(srv_gql.with_projects_disabled())
    }

    /// The project information
    async fn project(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<ProjectGQL> {
        let loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();

        let prj = loader.load_one(self.project_id).await?.ok_or_else(|| {
            GQLError::new("Project not found", GQLErrorCode::ProjectNotFound)
                .extend(|e| e.set("id", self.project_id.to_string()))
        })?;

        let prj_gql: ProjectGQL = prj.into();
        Ok(prj_gql
            .with_disable_server_fetch()
            .with_disable_collaboration_fetch())
    }
}

/// The collaboration invite target data
pub struct CollaborationInviteTargetGQL {
    /// The server ID
    server_id: showtimes_shared::ulid::Ulid,
    /// The project ID, if [`None`] then the source project will be duplicated
    project_id: Option<showtimes_shared::ulid::Ulid>,
}

#[Object]
impl CollaborationInviteTargetGQL {
    /// The server information
    async fn server(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<ServerGQL> {
        let loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();

        let srv = loader.load_one(self.server_id).await?.ok_or_else(|| {
            GQLError::new("Server not found", GQLErrorCode::ServerNotFound)
                .extend(|e| e.set("id", self.server_id.to_string()))
        })?;

        let srv_gql: ServerGQL = srv.into();
        Ok(srv_gql.with_projects_disabled())
    }

    /// The project information
    ///
    /// When not provided, this will be duplicated from the source
    /// server when the invite is accepted.
    async fn project(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Option<ProjectGQL>> {
        match self.project_id {
            None => Ok(None),
            Some(id) => {
                let loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();

                let prj = loader.load_one(id).await?.ok_or_else(|| {
                    GQLError::new("Project not found", GQLErrorCode::ProjectNotFound)
                        .extend(|e| e.set("id", id.to_string()))
                })?;

                let prj_gql: ProjectGQL = prj.into();
                Ok(Some(
                    prj_gql
                        .with_disable_server_fetch()
                        .with_disable_collaboration_fetch(),
                ))
            }
        }
    }
}

/// Collaboration sync information
pub struct CollaborationInviteGQL {
    /// The invite ID
    id: showtimes_shared::ulid::Ulid,
    /// The source
    source: CollaborationInviteSourceGQL,
    /// The target
    target: CollaborationInviteTargetGQL,
    /// Creation time
    created: chrono::DateTime<chrono::Utc>,
    /// Updated time
    updated: chrono::DateTime<chrono::Utc>,
}

#[Object]
impl CollaborationInviteGQL {
    /// The invite ID
    async fn id(&self) -> UlidGQL {
        self.id.into()
    }

    /// The source server and project
    async fn source(&self) -> &CollaborationInviteSourceGQL {
        &self.source
    }

    /// The target server and project
    async fn target(&self) -> &CollaborationInviteTargetGQL {
        &self.target
    }

    /// Creation time of the invite
    async fn created(&self) -> DateTimeGQL {
        self.created.into()
    }

    /// Last updated time of the invite
    async fn updated(&self) -> DateTimeGQL {
        self.updated.into()
    }
}

impl From<showtimes_db::m::ServerCollaborationSync> for CollaborationSyncGQL {
    fn from(db: showtimes_db::m::ServerCollaborationSync) -> Self {
        CollaborationSyncGQL {
            id: db.id,
            server_ids: db.projects.iter().map(|p| p.server).collect(),
            project_ids: db.projects.iter().map(|p| p.project).collect(),
            created: db.created,
            updated: db.updated,
            requester: None,
        }
    }
}

impl From<&showtimes_db::m::ServerCollaborationSync> for CollaborationSyncGQL {
    fn from(db: &showtimes_db::m::ServerCollaborationSync) -> Self {
        CollaborationSyncGQL {
            id: db.id,
            server_ids: db.projects.iter().map(|p| p.server).collect(),
            project_ids: db.projects.iter().map(|p| p.project).collect(),
            created: db.created,
            updated: db.updated,
            requester: None,
        }
    }
}

impl From<showtimes_db::m::ServerCollaborationInvite> for CollaborationInviteGQL {
    fn from(db: showtimes_db::m::ServerCollaborationInvite) -> Self {
        CollaborationInviteGQL {
            id: db.id,
            source: CollaborationInviteSourceGQL {
                server_id: db.source.server,
                project_id: db.source.project,
            },
            target: CollaborationInviteTargetGQL {
                server_id: db.target.server,
                project_id: db.target.project,
            },
            created: db.created,
            updated: db.updated,
        }
    }
}

impl From<&showtimes_db::m::ServerCollaborationInvite> for CollaborationInviteGQL {
    fn from(db: &showtimes_db::m::ServerCollaborationInvite) -> Self {
        CollaborationInviteGQL {
            id: db.id,
            source: CollaborationInviteSourceGQL {
                server_id: db.source.server,
                project_id: db.source.project,
            },
            target: CollaborationInviteTargetGQL {
                server_id: db.target.server,
                project_id: db.target.project,
            },
            created: db.created,
            updated: db.updated,
        }
    }
}
