//! A project models list

use async_graphql::{Enum, Object, SimpleObject, dataloader::DataLoader};
use errors::GQLError;
use showtimes_derive::EnumName;
use showtimes_gql_common::data_loader::{
    ServerDataLoader, ServerSyncIds, ServerSyncLoader, UserDataLoader,
};
use showtimes_gql_common::*;

use super::{
    collaborations::{CollaborationSyncGQL, CollaborationSyncRequester},
    servers::ServerGQL,
    users::UserGQL,
};

/// Enum to hold project types or kinds.
#[derive(Enum, Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[graphql(
    remote = "showtimes_db::m::ProjectType",
    rename_items = "SCREAMING_SNAKE_CASE"
)]
pub enum ProjectTypeGQL {
    /// The project is a movie.
    Movies,
    /// The project is a series, this is the default.
    Series,
    /// Oneshots of a series.
    #[graphql(name = "OVA")]
    OVAs,
    /// The project is a standard literature books.
    Books,
    /// The project is a manga.
    Manga,
    /// The project is a light novel.
    LightNovel,
    /// The project is a standard games.
    Games,
    /// The project is a visual novel.
    VisualNovel,
    /// The project is an unknown type.
    Unknown,
}

/// Enum to hold project status.
#[derive(Enum, Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, EnumName)]
#[graphql(
    remote = "showtimes_db::m::ProjectStatus",
    rename_items = "SCREAMING_SNAKE_CASE"
)]
pub enum ProjectStatusGQL {
    /// The project is currently ongoing or active.
    ///
    /// User can do any changes in the project.
    Active,
    /// The project is currently paused or in hiatus.
    ///
    /// User can do any changes in the project.
    /// This will give better user experience, when project is stalled.
    Paused,
    /// The current project is dropped or archived.
    ///
    /// Determining archive or dropped status:
    /// - When all episodes/chapters in the project are finished and released, the project is archived.
    /// - Otherwise, the project is dropped.
    ///
    /// User can't do any changes in the project.
    Archived,
}

/// The project poster information
#[derive(SimpleObject)]
pub struct PosterGQL {
    /// The poster metadata information
    image: ImageMetadataGQL,
    /// The int32 color value of the poster
    color: Option<u32>,
}

/// The role information on the project
#[derive(SimpleObject, Clone)]
pub struct RoleGQL {
    /// The order of the role, used for sorting
    order: i32,
    /// The role kind, this is always uppercased
    key: String,
    /// The role actual long name
    name: String,
}

impl PartialEq for RoleGQL {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for RoleGQL {}

impl PartialOrd for RoleGQL {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RoleGQL {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.order.cmp(&other.order)
    }
}

/// The status of an episode for a role on the project
#[derive(SimpleObject, Clone)]
pub struct RoleStatusGQL {
    /// The role information
    role: RoleGQL,
    /// The status of the role
    finished: bool,
}

impl RoleStatusGQL {
    fn with_role(role: &showtimes_db::m::Role, finished: bool) -> Self {
        RoleStatusGQL {
            role: role.into(),
            finished,
        }
    }
}

/// The assignee or someone who is assigned to a role
///
/// This is mapped to a user in the system.
pub struct RoleAssigneeGQL {
    /// The role information
    role: RoleGQL,
    /// The user ID
    user: Option<showtimes_shared::ulid::Ulid>,
}

#[Object]
impl RoleAssigneeGQL {
    /// The role information
    async fn role(&self) -> RoleGQL {
        self.role.clone()
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

/// An episode or chapter or progress in a project
#[derive(SimpleObject, Clone)]
#[graphql(rename_fields = "camelCase")]
pub struct ProjectProgressGQL {
    /// The episode or chapter number.
    ///
    /// For any project like Movie, VN, Game, or OVA. This might only be a single episode.
    number: u64,
    /// Is the progress finished or released.
    finished: bool,
    /// The air date of the episode or chapter.
    air_date: Option<DateTimeGQL>,
    /// The list of roles and their status for the episode.
    statuses: Vec<RoleStatusGQL>,
    /// The delay reason for this episode.
    delay_reason: Option<String>,
}

/// The project information
pub struct ProjectGQL {
    id: showtimes_shared::ulid::Ulid,
    title: String,
    poster: showtimes_db::m::Poster,
    roles: Vec<showtimes_db::m::Role>,
    progress: Vec<showtimes_db::m::EpisodeProgress>,
    assignees: Vec<showtimes_db::m::RoleAssignee>,
    integrations: Vec<showtimes_db::m::IntegrationId>,
    creator: showtimes_shared::ulid::Ulid,
    kind: showtimes_db::m::ProjectType,
    status: showtimes_db::m::ProjectStatus,
    created: chrono::DateTime<chrono::Utc>,
    updated: chrono::DateTime<chrono::Utc>,
    disable_server_fetch: bool,
    disable_collaboration_fetch: bool,
}

#[Object]
impl ProjectGQL {
    /// The project ID
    async fn id(&self) -> UlidGQL {
        self.id.into()
    }

    /// The project title
    async fn title(&self) -> String {
        self.title.clone()
    }

    /// The project poster
    async fn poster(&self) -> PosterGQL {
        self.poster.clone().into()
    }

    /// The project progress, this can fails if the roles are not found
    async fn progress(
        &self,
        #[graphql(
            name = "limitLatest",
            desc = "Get X latest of episode to be returned in status. If not provided all will be returned.",
            validator(minimum = 1, maximum = 10)
        )]
        limit_latest: Option<u32>,
        #[graphql(
            name = "returnLast",
            desc = "Always return the last episode when there is no progress left. Used in combination with `limitLatest`.",
            default = true
        )]
        return_last: bool,
    ) -> async_graphql::Result<Vec<ProjectProgressGQL>> {
        let mut progress = vec![];

        for p in &self.progress {
            progress.push(ProjectProgressGQL::from_db(p.clone(), self.roles.clone())?);
        }

        progress.sort_by(|a, b| a.number.cmp(&b.number));

        let actual_progress: Vec<ProjectProgressGQL> = if let Some(limit) = limit_latest {
            // Shift amount to the right
            let unreleased_idx = progress.iter().position(|p| !p.finished);
            let last_progress = progress.last();

            match unreleased_idx {
                Some(idx) => {
                    let end_idx = idx + limit as usize;
                    let end_idx = end_idx.min(progress.len());
                    let results = progress[idx..end_idx].to_vec();

                    if return_last && results.is_empty() {
                        // Return the last one
                        if let Some(progress) = last_progress {
                            vec![progress.clone()]
                        } else {
                            vec![]
                        }
                    } else {
                        results
                    }
                }
                None => {
                    if return_last {
                        // Return the last one
                        if let Some(progress) = last_progress {
                            vec![progress.clone()]
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    }
                }
            }
        } else {
            progress
        };

        Ok(actual_progress)
    }

    /// The project assignees or people that are working on it
    async fn assignees(&self) -> async_graphql::Result<Vec<RoleAssigneeGQL>> {
        let mut assignees = vec![];

        for assignee in &self.assignees {
            let role = get_role(&self.roles, assignee.key())?;
            assignees.push(RoleAssigneeGQL {
                role: role.into(),
                user: assignee.actor(),
            });
        }

        assignees.sort_by(|a, b| a.role.cmp(&b.role));

        Ok(assignees)
    }

    /// The project integrations information.
    ///
    /// Can be used to link to other services like Discord or FansubDB.
    async fn integrations(&self) -> Vec<IntegrationIdGQL> {
        self.integrations.iter().map(|i| i.into()).collect()
    }

    /// The project creator or the server that created the project.
    ///
    /// If the server is not found, this will throw an error.
    async fn creator(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<ServerGQL> {
        if self.disable_server_fetch {
            return GQLError::new(
                "Server fetch from this context is disabled to avoid looping",
                GQLErrorCode::ServerFetchDisabled,
            )
            .extend(|e| {
                e.set("id", self.id.to_string());
                e.set("server_id", self.creator.to_string());
                e.set("root", "project");
            })
            .into();
        }

        let loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();

        let server = loader.load_one(self.creator).await?.ok_or_else(|| {
            GQLError::new("Server not found", GQLErrorCode::ServerNotFound).extend(|e| {
                e.set("id", self.creator.to_string());
                e.set("project_id", self.id.to_string());
                e.set("root", "project");
            })
        })?;

        let map_server: ServerGQL = server.into();
        Ok(map_server.with_projects_disabled())
    }

    /// The project first airing date, taken from the first episode.
    #[graphql(name = "startTime")]
    async fn start_time(&self) -> Option<DateTimeGQL> {
        let mut sorted_progress = self.progress.clone();
        sorted_progress.sort_by(|a, b| a.number.cmp(&b.number));

        sorted_progress
            .first()
            .and_then(|p| p.aired.map(|d| d.into()))
    }

    /// The project episode/progress total count.
    async fn count(&self) -> u64 {
        self.progress.len() as u64
    }

    /// The project kind or type.
    async fn kind(&self) -> ProjectTypeGQL {
        self.kind.into()
    }

    /// The project status
    async fn status(&self) -> ProjectStatusGQL {
        self.status.into()
    }

    /// The project collaboration information
    async fn collaboration(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Option<CollaborationSyncGQL>> {
        if self.disable_collaboration_fetch {
            return Err(
                "Collaboration fetch from this context is disabled to avoid looping".into(),
            );
        }

        let loader = ctx.data_unchecked::<DataLoader<ServerSyncLoader>>();

        let res = loader
            .load_one(ServerSyncIds::new(self.creator, self.id))
            .await?;

        match res {
            Some(sync) => {
                let mapped = CollaborationSyncGQL::new(
                    &sync,
                    CollaborationSyncRequester::new(self.creator, self.id),
                );
                Ok(Some(mapped))
            }
            None => Ok(None),
        }
    }

    /// The project creation date
    async fn created(&self) -> DateTimeGQL {
        self.created.into()
    }

    /// The project last update date
    async fn updated(&self) -> DateTimeGQL {
        self.updated.into()
    }
}

impl From<showtimes_db::m::Role> for RoleGQL {
    fn from(role: showtimes_db::m::Role) -> Self {
        RoleGQL {
            order: role.order(),
            key: role.key().to_uppercase(),
            name: role.name().to_string(),
        }
    }
}

impl ProjectProgressGQL {
    fn from_db(
        progress: showtimes_db::m::EpisodeProgress,
        roles: Vec<showtimes_db::m::Role>,
    ) -> async_graphql::Result<Self> {
        let mut statuses = vec![];

        // XXX: We need to do this manually because we need to propagate the error.
        // XXX: Since `.try_collect()` is still nightly only :pensive:
        for status in &progress.statuses {
            let role = get_role(&roles, status.key())?;
            statuses.push(RoleStatusGQL::with_role(role, status.finished()));
        }

        statuses.sort_by(|a, b| a.role.cmp(&b.role));

        Ok(ProjectProgressGQL {
            number: progress.number,
            finished: progress.finished,
            air_date: progress.aired.map(|d| d.into()),
            delay_reason: progress.delay_reason.clone(),
            statuses,
        })
    }
}

impl From<&showtimes_db::m::Role> for RoleGQL {
    fn from(role: &showtimes_db::m::Role) -> Self {
        RoleGQL {
            order: role.order(),
            key: role.key().to_uppercase(),
            name: role.name().to_string(),
        }
    }
}

impl From<showtimes_db::m::Poster> for PosterGQL {
    fn from(poster: showtimes_db::m::Poster) -> Self {
        PosterGQL {
            image: poster.image.into(),
            color: poster.color,
        }
    }
}

impl From<&showtimes_db::m::Poster> for PosterGQL {
    fn from(poster: &showtimes_db::m::Poster) -> Self {
        PosterGQL {
            image: poster.image.clone().into(),
            color: poster.color,
        }
    }
}

impl From<showtimes_db::m::Project> for ProjectGQL {
    fn from(project: showtimes_db::m::Project) -> Self {
        ProjectGQL {
            id: project.id,
            title: project.title,
            poster: project.poster,
            roles: project.roles,
            progress: project.progress,
            assignees: project.assignees,
            integrations: project.integrations,
            creator: project.creator,
            kind: project.kind,
            status: project.status,
            created: project.created,
            updated: project.updated,
            disable_server_fetch: false,
            disable_collaboration_fetch: false,
        }
    }
}

impl From<&showtimes_db::m::Project> for ProjectGQL {
    fn from(project: &showtimes_db::m::Project) -> Self {
        ProjectGQL {
            id: project.id,
            title: project.title.clone(),
            poster: project.poster.clone(),
            roles: project.roles.clone(),
            progress: project.progress.clone(),
            assignees: project.assignees.clone(),
            integrations: project.integrations.clone(),
            creator: project.creator,
            kind: project.kind,
            status: project.status,
            created: project.created,
            updated: project.updated,
            disable_server_fetch: false,
            disable_collaboration_fetch: false,
        }
    }
}

impl ProjectGQL {
    /// Disable server fetch
    pub fn with_disable_server_fetch(mut self) -> Self {
        self.disable_server_fetch = true;
        self
    }

    /// Disable project collaboration fetch
    pub fn with_disable_collaboration_fetch(mut self) -> Self {
        self.disable_collaboration_fetch = true;
        self
    }
}

fn get_role(
    roles: &[showtimes_db::m::Role],
    key: impl Into<String>,
) -> async_graphql::Result<&showtimes_db::m::Role> {
    let key = key.into();
    roles.iter().find(|&r| r.key() == key).ok_or(
        GQLError::new(
            format!("Role {key} not found"),
            GQLErrorCode::ProjectRoleNotFound,
        )
        .extend(|e| {
            e.set("key", key);
            e.set(
                "roles",
                roles.iter().map(|r| r.key()).collect::<Vec<&str>>(),
            );
        })
        .build(),
    )
}
