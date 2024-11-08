use async_graphql::{dataloader::DataLoader, Enum, Object, SimpleObject};

use errors::GQLError;
use showtimes_gql_common::{
    data_loader::{ProjectDataLoader, UserDataLoader},
    *,
};
use showtimes_gql_models::{
    projects::{ProjectGQL, ProjectStatusGQL, RoleGQL},
    users::UserGQL,
};

/// A project created event
pub struct ProjectCreatedEventDataGQL {
    id: showtimes_shared::ulid::Ulid,
    title: String,
}

#[Object]
impl ProjectCreatedEventDataGQL {
    /// The project ID
    async fn id(&self) -> UlidGQL {
        self.id.into()
    }

    /// The project title
    async fn title(&self) -> &str {
        &self.title
    }

    /// The project information
    async fn project(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<ProjectGQL> {
        let loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();

        let project = loader.load_one(self.id).await?.ok_or_else(|| {
            GQLError::new("Project not found", GQLErrorCode::ProjectNotFound)
                .extend(|e| e.set("id", self.id.to_string()))
                .build()
        })?;

        let prj_gql = ProjectGQL::from(project);
        Ok(prj_gql
            .with_disable_server_fetch()
            .with_disable_collaboration_fetch())
    }
}

/// A project updated episode status
#[derive(Enum, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[graphql(
    remote = "showtimes_events::m::ProjectUpdatedEpisodeStatus",
    rename_items = "SCREAMING_SNAKE_CASE"
)]
pub enum ProjectUpdatedEpisodeStatusGQL {
    /// New episode added
    New,
    /// Episode removed
    Removed,
    /// Episode updated
    #[default]
    Updated,
}

/// A tiny information about episode update data event
///
/// Used in conjuction with the [`ProjectEpisodeUpdatedEvent`]
#[derive(SimpleObject)]
pub struct ProjectUpdatedEpisodeDataEventGQL {
    /// Episode number in the project
    number: u64,
    /// Unix timestamp of the episode
    aired: Option<i64>,
    /// Episode delay reason
    delay_reason: Option<String>,
    /// Episode status
    status: ProjectUpdatedEpisodeStatusGQL,
}

/// The assignee or someone who is assigned to a role
///
/// This is mapped to a user in the system.
pub struct ProjectUpdatedEventDataRoleAssigneeGQL {
    /// The role key
    key: String,
    /// The user ID
    user: Option<showtimes_shared::ulid::Ulid>,
}

#[Object]
impl ProjectUpdatedEventDataRoleAssigneeGQL {
    /// The role key
    async fn key(&self) -> &str {
        &self.key
    }

    /// The user information, this can be `None` if user not assigned.
    ///
    /// This will also silently return `None` if the user is not found.
    async fn user(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Option<UserGQL>> {
        match self.user {
            None => Ok(None),
            Some(user) => {
                let loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();

                match loader.load_one(user).await? {
                    Some(user) => {
                        let user_map: UserGQL = user.into();
                        Ok(Some(user_map.with_disable_server_fetch()))
                    }
                    None => Ok(None),
                }
            }
        }
    }
}

/// The data that contains the project updated information
///
/// Used in conjuction with the [`ProjectUpdatedEventDataGQL`]
///
/// Not all fields will be present, only the fields that have been updated
#[derive(SimpleObject)]
pub struct ProjectUpdatedEventDataContentGQL {
    /// The change in the project title
    title: Option<String>,
    /// The change in the project integrations
    integrations: Option<Vec<IntegrationIdGQL>>,
    /// The change in the assignees of the project
    assignees: Option<Vec<ProjectUpdatedEventDataRoleAssigneeGQL>>,
    /// The change in the roles of the project
    roles: Option<Vec<RoleGQL>>,
    /// The change in the project poster image
    poster_image: Option<ImageMetadataGQL>,
    /// The change in the project aliases
    aliases: Option<Vec<String>>,
    /// The change in the project progress
    progress: Option<Vec<ProjectUpdatedEpisodeDataEventGQL>>,
    /// The change in project status
    status: Option<ProjectStatusGQL>,
}

/// A project updated event
pub struct ProjectUpdatedEventDataGQL {
    id: showtimes_shared::ulid::Ulid,
    before: showtimes_events::m::ProjectUpdatedDataEvent,
    after: showtimes_events::m::ProjectUpdatedDataEvent,
}

#[Object]
impl ProjectUpdatedEventDataGQL {
    /// The project ID
    async fn id(&self) -> UlidGQL {
        self.id.into()
    }

    /// The project information
    async fn project(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<ProjectGQL> {
        let loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();

        let project = loader.load_one(self.id).await?.ok_or_else(|| {
            GQLError::new("Project not found", GQLErrorCode::ProjectNotFound)
                .extend(|e| e.set("id", self.id.to_string()))
                .build()
        })?;

        let prj_gql = ProjectGQL::from(project);
        Ok(prj_gql
            .with_disable_server_fetch()
            .with_disable_collaboration_fetch())
    }

    /// The project data before the update
    async fn before(&self) -> ProjectUpdatedEventDataContentGQL {
        ProjectUpdatedEventDataContentGQL::from(&self.before)
    }

    /// The project data after the update
    async fn after(&self) -> ProjectUpdatedEventDataContentGQL {
        ProjectUpdatedEventDataContentGQL::from(&self.after)
    }
}

/// The status of an episode for a role on the project
#[derive(SimpleObject)]
pub struct ProjectEpisodeEventDataRoleStatusGQL {
    /// The role key
    key: String,
    /// The role status
    finished: bool,
}

/// A project episode event
pub struct ProjectEpisodeUpdatedEventDataGQL {
    id: showtimes_shared::ulid::Ulid,
    number: u64,
    finished: Option<bool>,
    before: Vec<showtimes_db::m::RoleStatus>,
    after: Vec<showtimes_db::m::RoleStatus>,
    silent: bool,
}

#[Object]
impl ProjectEpisodeUpdatedEventDataGQL {
    /// The project ID
    async fn id(&self) -> UlidGQL {
        self.id.into()
    }

    /// The project information
    async fn project(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<ProjectGQL> {
        let loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();

        let project = loader.load_one(self.id).await?.ok_or_else(|| {
            GQLError::new("Project not found", GQLErrorCode::ProjectNotFound)
                .extend(|e| e.set("id", self.id.to_string()))
                .build()
        })?;

        let prj_gql = ProjectGQL::from(project);
        Ok(prj_gql
            .with_disable_server_fetch()
            .with_disable_collaboration_fetch())
    }

    /// The episode/progress number
    async fn number(&self) -> u64 {
        self.number
    }

    /// The episode finished status
    async fn finished(&self) -> Option<bool> {
        self.finished
    }

    /// The episode status before the update
    async fn before(&self) -> Vec<ProjectEpisodeEventDataRoleStatusGQL> {
        self.before.iter().map(|v| v.into()).collect()
    }

    /// The episode status after the update
    async fn after(&self) -> Vec<ProjectEpisodeEventDataRoleStatusGQL> {
        self.after.iter().map(|v| v.into()).collect()
    }

    /// This is silent update, if true, the event should not be broadcasted
    /// when receiving this event, the client should silently update the data
    async fn silent(&self) -> bool {
        self.silent
    }
}

/// A project deleted event
#[derive(SimpleObject)]
pub struct ProjectDeletedEventDataGQL {
    /// The project ID that was deleted
    id: UlidGQL,
}

impl From<showtimes_events::m::ProjectCreatedEvent> for ProjectCreatedEventDataGQL {
    fn from(value: showtimes_events::m::ProjectCreatedEvent) -> Self {
        Self {
            id: value.id(),
            title: value.title().to_string(),
        }
    }
}

impl From<&showtimes_events::m::ProjectCreatedEvent> for ProjectCreatedEventDataGQL {
    fn from(value: &showtimes_events::m::ProjectCreatedEvent) -> Self {
        Self {
            id: value.id(),
            title: value.title().to_string(),
        }
    }
}

impl From<showtimes_events::m::ProjectDeletedEvent> for ProjectDeletedEventDataGQL {
    fn from(value: showtimes_events::m::ProjectDeletedEvent) -> Self {
        Self {
            id: value.id().into(),
        }
    }
}

impl From<&showtimes_events::m::ProjectDeletedEvent> for ProjectDeletedEventDataGQL {
    fn from(value: &showtimes_events::m::ProjectDeletedEvent) -> Self {
        Self {
            id: value.id().into(),
        }
    }
}

impl From<showtimes_events::m::ProjectUpdatedEpisodeDataEvent>
    for ProjectUpdatedEpisodeDataEventGQL
{
    fn from(value: showtimes_events::m::ProjectUpdatedEpisodeDataEvent) -> Self {
        Self {
            number: value.number(),
            aired: value.aired(),
            delay_reason: value.delay_reason().map(|v| v.to_string()),
            status: value.status().into(),
        }
    }
}

impl From<&showtimes_events::m::ProjectUpdatedEpisodeDataEvent>
    for ProjectUpdatedEpisodeDataEventGQL
{
    fn from(value: &showtimes_events::m::ProjectUpdatedEpisodeDataEvent) -> Self {
        Self {
            number: value.number(),
            aired: value.aired(),
            delay_reason: value.delay_reason().map(|v| v.to_string()),
            status: value.status().into(),
        }
    }
}

impl From<&showtimes_db::m::RoleAssignee> for ProjectUpdatedEventDataRoleAssigneeGQL {
    fn from(value: &showtimes_db::m::RoleAssignee) -> Self {
        Self {
            key: value.key().to_string(),
            user: value.actor(),
        }
    }
}

impl From<showtimes_events::m::ProjectUpdatedDataEvent> for ProjectUpdatedEventDataContentGQL {
    fn from(value: showtimes_events::m::ProjectUpdatedDataEvent) -> Self {
        Self {
            title: value.title().map(|v| v.to_string()),
            integrations: value
                .integrations()
                .map(|v| v.iter().map(|v| v.into()).collect()),
            assignees: value
                .assignees()
                .map(|v| v.iter().map(|v| v.into()).collect()),
            roles: value.roles().map(|v| v.iter().map(|v| v.into()).collect()),
            poster_image: value.poster_image().map(|v| v.clone().into()),
            aliases: value.aliases().map(|v| v.to_vec()),
            progress: value
                .progress()
                .map(|v| v.iter().map(|v| v.into()).collect()),
            status: value.status().map(|v| v.into()),
        }
    }
}

impl From<&showtimes_events::m::ProjectUpdatedDataEvent> for ProjectUpdatedEventDataContentGQL {
    fn from(value: &showtimes_events::m::ProjectUpdatedDataEvent) -> Self {
        Self {
            title: value.title().map(|v| v.to_string()),
            integrations: value
                .integrations()
                .map(|v| v.iter().map(|v| v.into()).collect()),
            assignees: value
                .assignees()
                .map(|v| v.iter().map(|v| v.into()).collect()),
            roles: value.roles().map(|v| v.iter().map(|v| v.into()).collect()),
            poster_image: value.poster_image().map(|v| v.clone().into()),
            aliases: value.aliases().map(|v| v.to_vec()),
            progress: value
                .progress()
                .map(|v| v.iter().map(|v| v.into()).collect()),
            status: value.status().map(|v| v.into()),
        }
    }
}

impl From<showtimes_events::m::ProjectUpdatedEvent> for ProjectUpdatedEventDataGQL {
    fn from(value: showtimes_events::m::ProjectUpdatedEvent) -> Self {
        Self {
            id: value.id(),
            before: value.before().clone(),
            after: value.after().clone(),
        }
    }
}

impl From<&showtimes_events::m::ProjectUpdatedEvent> for ProjectUpdatedEventDataGQL {
    fn from(value: &showtimes_events::m::ProjectUpdatedEvent) -> Self {
        Self {
            id: value.id(),
            before: value.before().clone(),
            after: value.after().clone(),
        }
    }
}

impl From<showtimes_db::m::RoleStatus> for ProjectEpisodeEventDataRoleStatusGQL {
    fn from(value: showtimes_db::m::RoleStatus) -> Self {
        Self {
            key: value.key().to_string(),
            finished: value.finished(),
        }
    }
}

impl From<&showtimes_db::m::RoleStatus> for ProjectEpisodeEventDataRoleStatusGQL {
    fn from(value: &showtimes_db::m::RoleStatus) -> Self {
        Self {
            key: value.key().to_string(),
            finished: value.finished(),
        }
    }
}

impl From<showtimes_events::m::ProjectEpisodeUpdatedEvent> for ProjectEpisodeUpdatedEventDataGQL {
    fn from(value: showtimes_events::m::ProjectEpisodeUpdatedEvent) -> Self {
        Self {
            id: value.id(),
            number: value.number(),
            finished: value.finished(),
            before: value.before().to_vec(),
            after: value.after().to_vec(),
            silent: value.silent(),
        }
    }
}

impl From<&showtimes_events::m::ProjectEpisodeUpdatedEvent> for ProjectEpisodeUpdatedEventDataGQL {
    fn from(value: &showtimes_events::m::ProjectEpisodeUpdatedEvent) -> Self {
        Self {
            id: value.id(),
            number: value.number(),
            finished: value.finished(),
            before: value.before().to_vec(),
            after: value.after().to_vec(),
            silent: value.silent(),
        }
    }
}
