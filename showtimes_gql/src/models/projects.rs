use super::prelude::*;
use async_graphql::Object;

pub struct ProjectGQL {
    id: showtimes_shared::ulid::Ulid,
}

#[Object]
impl ProjectGQL {
    async fn id(&self) -> UlidGQL {
        self.id.into()
    }
}

impl From<showtimes_db::m::Project> for ProjectGQL {
    fn from(project: showtimes_db::m::Project) -> Self {
        ProjectGQL { id: project.id }
    }
}

impl From<&showtimes_db::m::Project> for ProjectGQL {
    fn from(project: &showtimes_db::m::Project) -> Self {
        ProjectGQL { id: project.id }
    }
}
