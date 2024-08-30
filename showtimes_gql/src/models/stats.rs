use async_graphql::{dataloader::DataLoader, Object};

use crate::data_loader::{ProjectDataLoader, ServerOwnerId};

/// The stats object for projects
pub struct StatsProjectsGQL {
    projects: Vec<showtimes_db::m::Project>,
}

#[Object]
impl StatsProjectsGQL {
    /// The total projects
    async fn total(&self) -> u64 {
        self.projects.len() as u64
    }

    /// Total of finished projects
    ///
    /// This depends if all the project has "released" or "finished" status toggled
    async fn finished(&self) -> u64 {
        let finished_proj = self
            .projects
            .iter()
            .filter(|&p| p.progress.iter().all(|p| p.finished))
            .count();
        finished_proj as u64
    }

    /// Total of projects in progress
    ///
    /// Inverse of finished projects
    async fn unfinished(&self) -> u64 {
        let unfinished_proj = self
            .projects
            .iter()
            .filter(|&p| p.progress.iter().any(|p| !p.finished))
            .count();
        unfinished_proj as u64
    }
}

/// The stats object
pub struct StatsGQL {
    server: showtimes_db::m::Server,
}

#[Object]
impl StatsGQL {
    /// Owners count in the server
    async fn owners(&self) -> u64 {
        self.server.owners.len() as u64
    }

    /// Project stats in the server
    async fn projects(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<StatsProjectsGQL> {
        let loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();

        let projects_load = loader.load_one(ServerOwnerId::new(self.server.id)).await?;

        match projects_load {
            Some(projects) => Ok(StatsProjectsGQL::new(projects)),
            None => Ok(StatsProjectsGQL::stub()),
        }
    }
}

impl StatsGQL {
    pub fn new(server: showtimes_db::m::Server) -> Self {
        Self { server }
    }
}

impl StatsProjectsGQL {
    fn new(projects: Vec<showtimes_db::m::Project>) -> Self {
        Self { projects }
    }

    fn stub() -> Self {
        Self { projects: vec![] }
    }
}
