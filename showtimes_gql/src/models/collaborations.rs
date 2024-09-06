use async_graphql::{dataloader::DataLoader, Object};
use showtimes_shared::ulid::Ulid;

use crate::data_loader::{ProjectDataLoader, ProjectDataLoaderKey, ServerDataLoader};

use super::{prelude::*, projects::ProjectGQL, servers::ServerGQL};

#[derive(Debug, Clone, Copy)]
pub struct CollaborationSyncRequester {
    server_id: showtimes_shared::ulid::Ulid,
    project_id: showtimes_shared::ulid::Ulid,
}

impl CollaborationSyncRequester {
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
    requester: CollaborationSyncRequester,
}

impl CollaborationSyncGQL {
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
            requester,
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
    async fn servers(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Vec<ServerGQL>> {
        let loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();

        let unmapped_servers_ids: Vec<Ulid> = self
            .server_ids
            .clone()
            .into_iter()
            .filter(|id| *id != self.requester.server_id)
            .collect();

        if unmapped_servers_ids.is_empty() {
            return Ok(vec![]);
        }

        let results_mapped: Vec<ServerGQL> = loader
            .load_many(unmapped_servers_ids)
            .await?
            .into_iter()
            .map(|(_, srv)| {
                let srv_gql: ServerGQL = srv.into();
                srv_gql.with_projects_disabled()
            })
            .collect();

        Ok(results_mapped)
    }

    /// All the attached projects
    async fn projects(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Vec<ProjectGQL>> {
        let loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();

        let unmapped_project_ids: Vec<ProjectDataLoaderKey> = self
            .project_ids
            .clone()
            .into_iter()
            .filter_map(|id| {
                if id != self.requester.project_id {
                    Some(ProjectDataLoaderKey::Id(id))
                } else {
                    None
                }
            })
            .collect();

        if unmapped_project_ids.is_empty() {
            return Ok(vec![]);
        }

        let results_mapped: Vec<ProjectGQL> = loader
            .load_many(unmapped_project_ids)
            .await?
            .into_iter()
            .map(|(_, proj)| {
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
