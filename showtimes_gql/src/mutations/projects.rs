use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

use async_graphql::{
    dataloader::DataLoader, CustomValidator, Enum, Error, ErrorExtensions, InputObject, Upload,
};
use chrono::TimeZone;
use showtimes_db::{m::UserKind, mongodb::bson::doc, DatabaseShared, ProjectHandler};
use showtimes_fs::FsPool;
use showtimes_metadata::m::AnilistMediaFormat;
use showtimes_search::SearchClientShared;
use tokio::{io::AsyncSeekExt, sync::Mutex};

use crate::{
    data_loader::{
        ProjectDataLoader, ProjectDataLoaderKey, ServerDataLoader, ServerSyncIds, ServerSyncLoader,
        UserDataLoader,
    },
    models::{
        prelude::{DateTimeGQL, OkResponse, UlidGQL},
        projects::ProjectGQL,
        search::ExternalSearchSource,
    },
};

use super::execute_search_events;

type ChangedEpisodes = BTreeMap<u64, TinyEpisodeChanges>;

#[derive(Debug, Clone, Default)]
struct TinyEpisodeChanges {
    finished: Option<bool>,
    before: Vec<showtimes_db::m::RoleStatus>,
    after: Vec<showtimes_db::m::RoleStatus>,
    silent: bool,
}

impl TinyEpisodeChanges {
    fn changed(&self) -> bool {
        self.finished.is_some() || !self.before.is_empty() || !self.after.is_empty()
    }
}

/// The input information of an external metadata source
#[derive(InputObject)]
pub struct ProjectCreateMetadataInputGQL {
    /// The ID of the external metadata source
    id: String,
    /// The kind of the external metadata source
    kind: ExternalSearchSource,
    /// Override "episode" count
    ///
    /// The data will be extrapolated automatically
    episode: Option<i32>,
    /// Override start date of the series or project
    #[graphql(name = "startDate")]
    start_date: Option<DateTimeGQL>,
}

struct ValidateRole;

impl CustomValidator<String> for ValidateRole {
    fn check(&self, value: &String) -> Result<(), async_graphql::InputValueError<String>> {
        if value.is_empty() {
            return Err(async_graphql::InputValueError::custom(format!(
                "Role `{}` key cannot be empty",
                &value
            )));
        };

        if !value.is_ascii() {
            return Err(async_graphql::InputValueError::custom(format!(
                "Role `{}` key must be ASCII",
                &value
            )));
        }

        if value.contains(' ') {
            return Err(async_graphql::InputValueError::custom(format!(
                "Role `{}` key cannot contain spaces",
                &value
            )));
        }

        if &value.to_ascii_uppercase() != value {
            return Err(async_graphql::InputValueError::custom(format!(
                "Role `{}` key must be uppercase",
                &value
            )));
        }

        Ok(())
    }
}

/// The input for roles information
#[derive(InputObject)]
pub struct ProjectRoleInputGQL {
    /// The role key
    #[graphql(validator(custom = "ValidateRole"))]
    key: String,
    /// The role long name
    #[graphql(validator(min_length = 1))]
    name: String,
}

/// The action for updating a role
#[derive(Enum, Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum ProjectRoleUpdateAction {
    /// Add new role
    Add,
    /// Remove a role
    Remove,
    /// Update a role
    Update,
}

/// The update information for a role
#[derive(InputObject)]
pub struct ProjectRoleUpdateInputGQL {
    /// The role itself
    role: ProjectRoleInputGQL,
    /// The action to perform for this role
    action: ProjectRoleUpdateAction,
}

/// The input assignees for a project
#[derive(InputObject)]
pub struct ProjectAssigneeInputGQL {
    /// The user ID
    id: UlidGQL,
    /// The role key
    #[graphql(validator(custom = "ValidateRole"))]
    role: String,
}

/// The update information for a assignee for a project
#[derive(InputObject)]
pub struct ProjectAssigneeUpdateInputGQL {
    /// The person itself
    ///
    /// To remove a person from a role, just set this to `None` or `null`
    id: Option<UlidGQL>,
    /// The role key
    #[graphql(validator(custom = "ValidateRole"))]
    role: String,
}

/// The update information of a status on each role of a progress
#[derive(InputObject)]
pub struct ProjectProgressStatusUpdateInputGQL {
    /// The role key
    #[graphql(validator(custom = "ValidateRole"))]
    role: String,
    /// The status of the role
    finished: bool,
}

/// The update information for a progress in the project
///
/// All fields optional except `number`, provide if you
/// want to update it.
///
/// This is only used in `projectUpdate` mutation, if you want to
/// remove or add episode manually use `projectProgressUpdate` mutation
#[derive(InputObject)]
pub struct ProjectProgressUpdateInputGQL {
    /// The episode number
    number: u64,
    /// Is episode finished or not
    finished: Option<bool>,
    /// The airing/release date of the episode/chapter.
    aired: Option<DateTimeGQL>,
    /// Delay reason, set `unsetDelay` to `true` to remove
    #[graphql(name = "delayReason", validator(min_length = 1))]
    delay_reason: Option<String>,
    /// Unset the delay reason, when set to `true` this will take
    /// precedence over `delayReason`
    #[graphql(name = "unsetDelay")]
    unset_delay: Option<bool>,
    /// The status of each role in the episode
    ///
    /// This will only update the status for the role that is provided
    /// in the list, to not update the status, just do not include the list
    statuses: Option<Vec<ProjectProgressStatusUpdateInputGQL>>,
    /// Do a silent update, this will not broadcast the event for status change
    #[graphql(default = false)]
    silent: bool,
}

impl ProjectProgressUpdateInputGQL {
    /// Check if any field is set
    fn is_any_set(&self) -> bool {
        self.finished.is_some()
            || self.aired.is_some()
            || self.delay_reason.is_some()
            || self.unset_delay.is_some()
            || self.statuses.is_some()
    }

    /// Check if any field is set that is not status
    fn is_any_set_except_status(&self) -> bool {
        self.finished.is_some()
            || self.aired.is_some()
            || self.delay_reason.is_some()
            || self.unset_delay.is_some()
    }
}

/// The input object for creating a new project
#[derive(InputObject)]
pub struct ProjectCreateInputGQL {
    /// The external metadata being used to create the project
    metadata: ProjectCreateMetadataInputGQL,
    /// The roles list for the project
    ///
    /// When not being provided, the system will automatically create new one depending on the project type
    roles: Option<Vec<ProjectRoleInputGQL>>,
    /// The assignees for the project
    ///
    /// To not assign anyone to specific (or all) roles, just do not include the role in this list
    assignees: Vec<ProjectAssigneeInputGQL>,
    /// The poster image
    poster: Option<Upload>,
    /// The poster color (rgb color in integer format)
    poster_color: Option<u32>,
    /// The aliases to be added to the project.
    ///
    /// If provided, the aliases from metadata will be ignored
    #[graphql(validator(custom = "super::NonEmptyValidator"))]
    aliases: Option<Vec<String>>,
}

/// The input object for update a project
///
/// All fields optional, only provide the fields that you want to update
///
/// The update will be done in this order:
/// 0. Update title, aliases, etc.
/// 1. Update the roles
/// 2. Update the assignees
/// 3. Sync metadata (if needed)
/// 4. Update the progress
/// 5. Update the poster + poster color
#[derive(InputObject)]
pub struct ProjectUpdateInputGQL {
    /// The title of the project
    #[graphql(validator(min_length = 1))]
    title: Option<String>,
    /// The aliases for the project
    ///
    /// Providing empty list will remove all aliases
    #[graphql(validator(custom = "super::NonEmptyValidator"))]
    aliases: Option<Vec<String>>,
    /// The modified roles list for the project
    roles: Option<Vec<ProjectRoleUpdateInputGQL>>,
    /// The modfied assignees for the project
    assignees: Option<Vec<ProjectAssigneeUpdateInputGQL>>,
    /// The poster image
    poster: Option<Upload>,
    /// The poster color (rgb color in integer format)
    poster_color: Option<u32>,
    /// Synchronize the project with external metadata
    ///
    /// This will update the progress count.
    ///
    /// The behaviour are like this:
    /// - When a new episode is found, it will be added to the end of the list
    /// - When an episode is removed, if no progress (or released), it will be removed
    /// - All episode will have their aired date updated
    ///
    /// Default to `false`
    #[graphql(default = false, name = "syncMetadata")]
    sync_metadata: bool,
    /// The progress update for the project
    ///
    /// This will only update the progress, to add or remove episode/chapter, use `projectProgressUpdate` mutation
    progress: Option<Vec<ProjectProgressUpdateInputGQL>>,
}

impl ProjectUpdateInputGQL {
    /// Check if any field is set
    fn is_any_set(&self) -> bool {
        self.title.is_some()
            || self.aliases.is_some()
            || self.roles.is_some()
            || self.assignees.is_some()
            || self.poster.is_some()
            || self.poster_color.is_some()
            || self.sync_metadata
            || self.progress.is_some()
    }
}

/// The input object for update a project progress count manually.
///
/// When a pre-existing episode is updated, the system will update the episode
/// instead of adding a new one.
///
/// Note: This can make the system out of sync with the external metadata.
#[derive(InputObject)]
pub struct ProgressCreateInputGQL {
    /// The episode title
    number: u64,
    /// Airing date of the progress
    aired: Option<DateTimeGQL>,
}

#[derive(Clone, Debug)]
struct ExternalMediaFetchProgressResult {
    number: u32,
    aired_at: Option<chrono::DateTime<chrono::Utc>>,
}

struct ExternalMediaFetchResult {
    title: String,
    integrations: Vec<showtimes_db::m::IntegrationId>,
    progress: Vec<ExternalMediaFetchProgressResult>,
    aliases: Vec<String>,
    kind: showtimes_db::m::ProjectType,
    poster_url: Option<String>,
}

fn unix_to_chrono(unix: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::<chrono::Utc>::from_timestamp(unix, 0).unwrap()
}

fn fuzzy_yyyy_mm_dd_to_chrono(date: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let parts: Vec<&str> = date.split('-').collect();
    let year: Option<i32> = parts.first().and_then(|y| y.parse().ok());
    let month: Option<u32> = parts.get(1).and_then(|m| m.parse().ok());
    let day: Option<u32> = parts.get(2).and_then(|d| d.parse().ok());

    // If all None, return None
    match (year, month, day) {
        (Some(year), Some(month), Some(day)) => chrono::Utc
            .with_ymd_and_hms(year, month, day, 0, 0, 0)
            .single(),
        _ => None,
    }
}

async fn fetch_metadata_via_anilist(
    ctx: &async_graphql::Context<'_>,
    input: &ProjectCreateMetadataInputGQL,
) -> async_graphql::Result<ExternalMediaFetchResult> {
    let id_fetch = input.id.parse::<i32>().map_err(|_| {
        Error::new("Invalid Anilist ID").extend_with(|_, e| e.set("id", input.id.clone()))
    })?;
    let anilist_loader = ctx.data_unchecked::<Arc<Mutex<showtimes_metadata::AnilistProvider>>>();
    let mut anilist = anilist_loader.lock().await;

    let anilist_info = anilist.get_media(id_fetch).await?;

    let mut integrations = vec![showtimes_db::m::IntegrationId::new(
        id_fetch.to_string(),
        showtimes_db::m::IntegrationType::ProviderAnilist,
    )];
    if let Some(mal_id) = anilist_info.id_mal {
        integrations.push(showtimes_db::m::IntegrationId::new(
            mal_id.to_string(),
            showtimes_db::m::IntegrationType::ProviderAnilistMal,
        ));
    }

    let ani_title = anilist_info.title.clone();
    let mut rest_titles: VecDeque<String> = VecDeque::new();
    if let Some(romaji_title) = ani_title.romaji {
        rest_titles.push_back(romaji_title);
    }
    if let Some(eng_title) = ani_title.english {
        rest_titles.push_back(eng_title);
    }
    if let Some(native_title) = ani_title.native {
        rest_titles.push_back(native_title);
    }

    // Remove the title from the rest of the titles
    let first_title = rest_titles.pop_front().ok_or_else(|| {
        Error::new("No titles found from fetching metadata").extend_with(|_, e| {
            e.set("reason", "no_titles");
            e.set("id", id_fetch);
        })
    })?;

    let mut merged_episodes: Vec<ExternalMediaFetchProgressResult> = vec![];
    let mut continue_fetch = true;
    let mut current_page = 1;
    while continue_fetch {
        let air_sched = anilist
            .get_airing_schedules(id_fetch, Some(current_page))
            .await?;
        let resultings: Vec<ExternalMediaFetchProgressResult> = air_sched
            .airing_schedules
            .iter()
            .map(|sched| ExternalMediaFetchProgressResult {
                number: sched.episode as u32,
                aired_at: Some(unix_to_chrono(sched.airing_at)),
            })
            .collect();

        merged_episodes.extend(resultings);

        if air_sched.page_info.has_next_page {
            continue_fetch = true;
            current_page += 1;
        } else {
            continue_fetch = false;
        }
    }

    let est_episode = match anilist_info.kind {
        showtimes_metadata::m::AnilistMediaType::Anime => {
            let episode_count = anilist_info
                .episodes
                .unwrap_or_else(|| input.episode.unwrap_or(0));
            if episode_count < 1 {
                return Err(
                    Error::new("No episodes found from fetching metadata").extend_with(|_, e| {
                        e.set("reason", "no_episodes");
                        e.set("id", id_fetch);
                    }),
                );
            }

            episode_count
        }
        showtimes_metadata::m::AnilistMediaType::Manga => anilist_info
            .chapters
            .unwrap_or_else(|| input.episode.unwrap_or(0)),
    };

    let start_time = match (anilist_info.start_date, input.start_date.clone()) {
        (_, Some(start_date)) => *start_date,
        (Some(fuzzy_start), _) => fuzzy_start.into_chrono().ok_or_else(|| {
            Error::new("Invalid fuzzy date from Anilist, please provide override").extend_with(
                |_, e| {
                    e.set("reason", "invalid_fuzzy_date");
                },
            )
        })?,
        _ => {
            return Err(
                Error::new("No start date found from fetching metadata").extend_with(|_, e| {
                    e.set("reason", "no_start_date");
                    e.set("id", id_fetch);
                }),
            );
        }
    };

    if merged_episodes.is_empty() && est_episode > 0 {
        // Extrapolate the episodes, we separate the episode start date by 1 week
        let mut current_time = start_time;
        for i in 1..=est_episode {
            merged_episodes.push(ExternalMediaFetchProgressResult {
                number: i as u32,
                aired_at: Some(current_time),
            });

            current_time += chrono::Duration::weeks(1);
        }
    }

    merged_episodes.sort_by(|a, b| a.number.cmp(&b.number));

    match anilist_info.format {
        showtimes_metadata::m::AnilistMediaFormat::Tv
        | showtimes_metadata::m::AnilistMediaFormat::TvShort
        | showtimes_metadata::m::AnilistMediaFormat::OVA
        | showtimes_metadata::m::AnilistMediaFormat::ONA => {
            // All of this requires episode information
            if merged_episodes.is_empty() {
                return Err(
                    Error::new("No episodes found from fetching metadata").extend_with(|_, e| {
                        e.set("reason", "no_episodes");
                        e.set("id", id_fetch);
                    }),
                );
            }

            // Check the episode range if all exists
            let first_ep = merged_episodes[0].clone();
            let last_ep = merged_episodes.clone().last().unwrap().clone();
            if first_ep.number > 1 {
                // Extrapolate backward, use this episode start as all the previous episodes
                // This handle weird situation with something like Frieren in Anilist
                for i in 1..first_ep.number {
                    merged_episodes.push(ExternalMediaFetchProgressResult {
                        number: i,
                        aired_at: first_ep.aired_at,
                    });
                }
            }

            let est_ep_u32: u32 = est_episode.try_into().unwrap();
            if last_ep.number < est_ep_u32 {
                // Extrapolate forward, use the last episode start as the basis
                let mut last_ep_start = last_ep.aired_at;
                for i in (last_ep.number + 1)..=est_ep_u32 {
                    let aired_at = match last_ep_start {
                        Some(last) => {
                            let pushed = last + chrono::Duration::weeks(1);
                            last_ep_start = Some(pushed);
                            Some(pushed)
                        }
                        None => None,
                    };
                    merged_episodes.push(ExternalMediaFetchProgressResult {
                        number: i,
                        aired_at,
                    });
                }
            }
        }
        _ => {
            // Everything else, create single entry
            if merged_episodes.is_empty() {
                merged_episodes.push(ExternalMediaFetchProgressResult {
                    number: 1,
                    aired_at: Some(start_time),
                });
            }
        }
    }

    // Final sort
    merged_episodes.sort_by(|a, b| a.number.cmp(&b.number));

    let media_kind = match anilist_info.format {
        AnilistMediaFormat::Tv | AnilistMediaFormat::TvShort | AnilistMediaFormat::ONA => {
            showtimes_db::m::ProjectType::Series
        }
        AnilistMediaFormat::Movie => showtimes_db::m::ProjectType::Movies,
        AnilistMediaFormat::Special | AnilistMediaFormat::OVA | AnilistMediaFormat::Music => {
            showtimes_db::m::ProjectType::OVAs
        }
        AnilistMediaFormat::Manga | AnilistMediaFormat::OneShot => {
            showtimes_db::m::ProjectType::Manga
        }
        AnilistMediaFormat::Novel => showtimes_db::m::ProjectType::LightNovel,
    };

    let poster_url = anilist_info.cover_image.get_image();

    Ok(ExternalMediaFetchResult {
        title: first_title,
        integrations,
        progress: merged_episodes,
        aliases: rest_titles.into_iter().collect(),
        kind: media_kind,
        poster_url,
    })
}

async fn fetch_metadata_via_vndb(
    ctx: &async_graphql::Context<'_>,
    input: &ProjectCreateMetadataInputGQL,
) -> async_graphql::Result<ExternalMediaFetchResult> {
    let vndb_loader = ctx.data_unchecked::<Arc<showtimes_metadata::VndbProvider>>();

    let input_id = &input.id;
    if input_id.starts_with("v") {
        return Err(Error::new("Invalid VNDB ID").extend_with(|_, e| e.set("id", input_id.clone())));
    }
    let id_test = input_id.trim_start_matches('v');
    if id_test.parse::<u64>().is_err() {
        return Err(Error::new("Invalid VNDB ID").extend_with(|_, e| e.set("id", input_id.clone())));
    }

    let vndb_info = vndb_loader.get(input_id).await?;

    let integrations = vec![showtimes_db::m::IntegrationId::new(
        input_id.clone(),
        showtimes_db::m::IntegrationType::ProviderVndb,
    )];

    let mut all_titles: Vec<String> = vec![];
    if let Some(main_title) = vndb_info.get_main_title() {
        all_titles.push(main_title);
    }
    if let Some(eng_title) = vndb_info.get_english_title() {
        all_titles.push(eng_title);
    }
    if let Some(original_title) = vndb_info.get_original_title() {
        all_titles.push(original_title.title);
        if let Some(latinized) = original_title.latin {
            all_titles.push(latinized);
        }
    }

    // Deduplicate the titles
    all_titles.dedup();

    // Convert to VecDeque
    let mut all_titles: VecDeque<String> = all_titles.into_iter().collect();

    // Remove the title from the rest of the titles
    let first_title = all_titles.pop_front().ok_or_else(|| {
        Error::new("No titles found from fetching metadata").extend_with(|_, e| {
            e.set("reason", "no_titles");
            e.set("id", input_id.clone());
        })
    })?;

    let aired_at = vndb_info
        .released
        .and_then(|d| fuzzy_yyyy_mm_dd_to_chrono(&d));

    let poster_url = vndb_info.image.url;

    Ok(ExternalMediaFetchResult {
        title: first_title,
        integrations,
        progress: vec![ExternalMediaFetchProgressResult {
            number: 1,
            aired_at,
        }],
        aliases: all_titles.into_iter().collect(),
        kind: showtimes_db::m::ProjectType::VisualNovel,
        poster_url: Some(poster_url),
    })
}

async fn fetch_metadata_via_tmdb(
    ctx: &async_graphql::Context<'_>,
    input: &ProjectCreateMetadataInputGQL,
) -> async_graphql::Result<ExternalMediaFetchResult> {
    // TMDb request for ID requires a prefix of movies: or tv: to be valid
    let input_id = input.id.parse::<i32>().map_err(|_| {
        Error::new("Invalid TMDb ID").extend_with(|_, e| e.set("id", input.id.clone()))
    })?;
    let tmdb_loader = ctx.data_unchecked::<Arc<showtimes_metadata::TMDbProvider>>();

    let tmdb_info = tmdb_loader.get_movie_details(input_id).await?;

    let integrations = vec![showtimes_db::m::IntegrationId::new(
        input_id.to_string(),
        showtimes_db::m::IntegrationType::ProviderTmdb,
    )];

    let mut all_titles: VecDeque<String> = VecDeque::new();

    if let Some(title) = tmdb_info.title.clone() {
        all_titles.push_back(title);
    }
    if let Some(orig_title) = tmdb_info.original_title.clone() {
        all_titles.push_back(orig_title);
    }

    let poster_url = tmdb_info.poster_url();

    // Remove the title from the rest of the titles
    let first_title = all_titles.pop_front().ok_or_else(|| {
        Error::new("No titles found from fetching metadata").extend_with(|_, e| {
            e.set("reason", "no_titles");
            e.set("id", input_id.to_string());
        })
    })?;

    let aired_at = tmdb_info
        .release_date
        .and_then(|d| fuzzy_yyyy_mm_dd_to_chrono(&d));

    Ok(ExternalMediaFetchResult {
        title: first_title,
        integrations,
        progress: vec![ExternalMediaFetchProgressResult {
            number: 1,
            aired_at,
        }],
        aliases: all_titles.into_iter().collect(),
        kind: showtimes_db::m::ProjectType::Movies,
        poster_url,
    })
}

enum ProjectEventsError {
    SearchError(showtimes_search::MeiliError),
    EventsError(showtimes_events::ClickHouseError),
}

impl From<showtimes_search::MeiliError> for ProjectEventsError {
    fn from(e: showtimes_search::MeiliError) -> Self {
        ProjectEventsError::SearchError(e)
    }
}

impl From<showtimes_events::ClickHouseError> for ProjectEventsError {
    fn from(e: showtimes_events::ClickHouseError) -> Self {
        ProjectEventsError::EventsError(e)
    }
}

impl std::fmt::Debug for ProjectEventsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectEventsError::SearchError(e) => write!(f, "{:?}", e),
            ProjectEventsError::EventsError(e) => write!(f, "{:?}", e),
        }
    }
}

impl std::fmt::Display for ProjectEventsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectEventsError::SearchError(e) => write!(f, "{}", e),
            ProjectEventsError::EventsError(e) => write!(f, "{}", e),
        }
    }
}

// TODO: Merge with `make_and_update_project`
async fn make_and_update_project_events(
    db: &showtimes_db::DatabaseShared,
    meili: &showtimes_search::SearchClientShared,
    project: &mut showtimes_db::m::Project,
    // JoinHandle return Result<(), dyn Error>
    task_events: &mut Vec<tokio::task::JoinHandle<Result<(), ProjectEventsError>>>,
) -> async_graphql::Result<ProjectGQL> {
    // Save the project
    let prj_handler = ProjectHandler::new(db);
    prj_handler.save(project, None).await?;

    // Update index
    let prj_clone = project.clone();
    let meili_clone = meili.clone();
    let task_search = tokio::task::spawn(async move {
        let prj_search = showtimes_search::models::Project::from(prj_clone);
        prj_search
            .update_document(&meili_clone)
            .await
            .map_err(|e| e.into())
    });
    task_events.push(task_search);

    let awaiter = futures::future::join_all(task_events).await;

    for await_res in awaiter {
        await_res??;
    }

    let prj_gql: ProjectGQL = project.clone().into();

    Ok(prj_gql)
}

async fn make_and_update_project(
    db: &showtimes_db::DatabaseShared,
    meili: &showtimes_search::SearchClientShared,
    project: &mut showtimes_db::m::Project,
) -> async_graphql::Result<ProjectGQL> {
    let mut stub = vec![];
    make_and_update_project_events(db, meili, project, &mut stub).await
}

async fn check_permissions(
    ctx: &async_graphql::Context<'_>,
    id: showtimes_shared::ulid::Ulid,
    user: &showtimes_db::m::User,
    project_id: Option<showtimes_shared::ulid::Ulid>,
) -> async_graphql::Result<showtimes_db::m::Server> {
    let srv_loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();

    let srv = srv_loader.load_one(id).await?;
    if srv.is_none() {
        return Err(Error::new("Server not found").extend_with(|_, e| {
            e.set("id", id.to_string());
            e.set("reason", "invalid_server");
        }));
    }

    let srv = srv.unwrap();
    let find_user = srv.owners.iter().find(|o| o.id == user.id);

    match (find_user, user.kind) {
        (Some(user), showtimes_db::m::UserKind::User) => {
            // Check if we are allowed to create a project
            match project_id {
                Some(project_id) => {
                    if user.privilege == showtimes_db::m::UserPrivilege::ProjectManager
                        && !user.has_id(project_id)
                    {
                        Err(
                            Error::new("User not allowed to update projects").extend_with(
                                |_, e| {
                                    e.set("id", id.to_string());
                                    e.set("reason", "invalid_privilege");
                                },
                            ),
                        )
                    } else {
                        Ok(srv)
                    }
                }
                None => {
                    if user.privilege < showtimes_db::m::UserPrivilege::Manager {
                        Err(
                            Error::new("User not allowed to create/delete projects").extend_with(
                                |_, e| {
                                    e.set("id", id.to_string());
                                    e.set("reason", "invalid_privilege");
                                },
                            ),
                        )
                    } else {
                        Ok(srv)
                    }
                }
            }
        }
        (None, showtimes_db::m::UserKind::User) => Err(Error::new(
            "User not allowed to create projects",
        )
        .extend_with(|_, e| {
            e.set("id", id.to_string());
            e.set("reason", "invalid_user");
        })),
        _ => {
            // Allow anyone to create a project
            Ok(srv)
        }
    }
}

pub async fn mutate_projects_create(
    ctx: &async_graphql::Context<'_>,
    user: showtimes_db::m::User,
    id: UlidGQL,
    input: ProjectCreateInputGQL,
) -> async_graphql::Result<ProjectGQL> {
    let usr_loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();
    let db = ctx.data_unchecked::<DatabaseShared>();
    let storages = ctx.data_unchecked::<Arc<FsPool>>();
    let meili = ctx.data_unchecked::<SearchClientShared>();

    // Check perms
    let srv = check_permissions(ctx, *id, &user, None).await?;

    // Fetch all assignees
    let assignee_keys = input
        .assignees
        .iter()
        .map(|a| *a.id)
        .collect::<Vec<showtimes_shared::ulid::Ulid>>();
    let all_assignees = usr_loader.load_many(assignee_keys).await?;

    // Fetch metadata
    let metadata = match input.metadata.kind {
        ExternalSearchSource::Anilist => fetch_metadata_via_anilist(ctx, &input.metadata).await?,
        ExternalSearchSource::Vndb => fetch_metadata_via_vndb(ctx, &input.metadata).await?,
        ExternalSearchSource::TMDb => fetch_metadata_via_tmdb(ctx, &input.metadata).await?,
    };

    let all_roles = match input.roles {
        Some(roles) => {
            let mut roles_list: Vec<showtimes_db::m::Role> = vec![];
            for (idx, role) in roles.iter().enumerate() {
                let role_map = showtimes_db::m::Role::new(role.key.clone(), role.name.clone())
                    .map_err(|e_root| {
                        Error::new("Failed to create role").extend_with(|_, e| {
                            e.set("reason", format!("{}", e_root));
                            e.set("index", idx);
                            e.set("key", role.key.clone());
                            e.set("name", role.name.clone());
                        })
                    })?;
                roles_list.push(role_map.with_order(idx as i32));
            }

            roles_list
        }
        None => {
            let proj_kind: showtimes_db::m::ProjectKind = metadata.kind.into();
            proj_kind.default_roles()
        }
    };

    // Create the assignees
    let mut assignees: Vec<showtimes_db::m::RoleAssignee> = vec![];
    for role in all_roles.iter() {
        let assignee = input.assignees.iter().find(|&a| a.role == role.key());

        match assignee {
            Some(assignee) => {
                let user_info = all_assignees.get(&*assignee.id);
                match user_info {
                    Some(user_info) => {
                        assignees.push(showtimes_db::m::RoleAssignee::new(
                            role.key(),
                            Some(user_info.id),
                        )?);
                    }
                    None => {
                        assignees.push(showtimes_db::m::RoleAssignee::new(role.key(), None)?);
                    }
                }
            }
            None => {
                assignees.push(showtimes_db::m::RoleAssignee::new(role.key(), None)?);
            }
        }
    }

    // Create the progress
    let mut all_progress: Vec<showtimes_db::m::EpisodeProgress> = vec![];
    for episode in metadata.progress {
        let mut progress = showtimes_db::m::EpisodeProgress::new_with_roles(
            episode.number as u64,
            false,
            &all_roles,
        );
        progress.set_aired(episode.aired_at);
        all_progress.push(progress);
    }

    let mut project = showtimes_db::m::Project::new(metadata.title, metadata.kind, srv.id)?;
    project.roles = all_roles;
    project.assignees = assignees;
    project.progress = all_progress;
    if let Some(aliases) = input.aliases {
        if aliases.is_empty() {
            project.aliases = metadata.aliases;
        } else {
            project.aliases = aliases;
        }
    } else {
        project.aliases = metadata.aliases;
    }
    project.integrations = metadata.integrations;

    // Upload the poster
    let poster_url = match (input.poster, metadata.poster_url) {
        (Some(poster), _) => {
            let info_up = poster.value(ctx)?;
            let mut file_target = tokio::fs::File::from_std(info_up.content);

            // Get format
            let format = crate::image::detect_upload_data(&mut file_target).await?;
            // Seek back to the start of the file
            file_target.seek(std::io::SeekFrom::Start(0)).await?;

            let filename = format!("cover.{}", format.as_extension());

            storages
                .file_stream_upload(
                    project.id,
                    &filename,
                    file_target,
                    Some(&srv.id.to_string()),
                    Some(showtimes_fs::FsFileKind::Images),
                )
                .await?;

            showtimes_db::m::ImageMetadata::new(
                showtimes_fs::FsFileKind::Images.as_path_name(),
                project.id,
                &filename,
                format.as_extension(),
                Some(srv.id.to_string()),
            )
        }
        (None, Some(poster)) => {
            let cover_bytes = download_cover(&poster).await?;

            let cover_format = poster.split('.').last().unwrap_or("jpg");

            let cover_key = format!("cover.{}", cover_format);

            let stream = std::io::Cursor::new(cover_bytes);

            storages
                .file_stream_upload(
                    project.id,
                    &cover_key,
                    stream,
                    Some(&srv.id.to_string()),
                    Some(showtimes_fs::FsFileKind::Images),
                )
                .await?;

            showtimes_db::m::ImageMetadata::new(
                showtimes_fs::FsFileKind::Images.as_path_name(),
                project.id,
                &cover_key,
                cover_format,
                Some(srv.id.to_string()),
            )
        }
        (None, None) => showtimes_db::m::ImageMetadata::new(
            showtimes_fs::FsFileKind::Invalids.as_path_name(),
            "project",
            "default.png",
            "png",
            None::<String>,
        ),
    };

    project.poster =
        showtimes_db::m::Poster::new_with_color(poster_url, input.poster_color.unwrap_or(16614485));

    // Create event commit tasks
    let event_ch = ctx
        .data_unchecked::<showtimes_events::SharedSHClickHouse>()
        .clone();
    let prj_clone = project.clone();
    let task_events = tokio::task::spawn(async move {
        event_ch
            .create_event(
                showtimes_events::m::EventKind::ProjectCreated,
                showtimes_events::m::ProjectCreatedEvent::from(&prj_clone),
                if user.kind == UserKind::Owner {
                    None
                } else {
                    Some(user.id.to_string())
                },
            )
            .await
            .map_err(|e| e.into())
    });

    // Save the project
    let mut events = vec![task_events];
    make_and_update_project_events(db, meili, &mut project, &mut events).await
}

async fn download_cover(url: &str) -> anyhow::Result<Vec<u8>> {
    let ua_ver = format!(
        "showtimes-rs-gql/{} (+https://github.com/naoTimesdev/showtimes-rs)",
        env!("CARGO_PKG_VERSION")
    );
    let mut header_maps = reqwest::header::HeaderMap::new();
    header_maps.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_str(&ua_ver)?,
    );

    let client = reqwest::ClientBuilder::new()
        .http2_adaptive_window(true)
        .default_headers(header_maps)
        .build()?;

    let resp = client.get(url).send().await?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("Failed to download cover"));
    }

    let bytes = resp.bytes().await?;
    let bytes_map = bytes.to_vec();
    Ok(bytes_map)
}

pub async fn mutate_projects_delete(
    ctx: &async_graphql::Context<'_>,
    user: showtimes_db::m::User,
    id: UlidGQL,
) -> async_graphql::Result<OkResponse> {
    let srv_loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();
    let prj_loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();
    let db = ctx.data_unchecked::<DatabaseShared>();
    let storages = ctx.data_unchecked::<Arc<FsPool>>();
    let meili = ctx.data_unchecked::<SearchClientShared>();

    let prj_info = prj_loader.load_one(ProjectDataLoaderKey::Id(*id)).await?;

    if prj_info.is_none() {
        return Err(Error::new("Project not found").extend_with(|_, e| {
            e.set("id", id.to_string());
            e.set("reason", "invalid_project");
        }));
    }

    let prj_info = prj_info.unwrap();

    let srv = srv_loader.load_one(prj_info.creator).await?;

    if srv.is_none() {
        return Err(Error::new("Server not found").extend_with(|_, e| {
            e.set("id", id.to_string());
            e.set("reason", "invalid_server");
        }));
    }

    let srv = srv.unwrap();
    let find_user = srv.owners.iter().find(|o| o.id == user.id);
    match (find_user, user.kind) {
        (Some(user), showtimes_db::m::UserKind::User) => {
            // Check if we are allowed to create a project
            if user.privilege < showtimes_db::m::UserPrivilege::Manager {
                return Err(
                    Error::new("User not allowed to create projects").extend_with(|_, e| {
                        e.set("id", id.to_string());
                        e.set("reason", "invalid_privilege");
                    }),
                );
            }
        }
        (None, showtimes_db::m::UserKind::User) => {
            return Err(
                Error::new("User not allowed to create projects").extend_with(|_, e| {
                    e.set("id", id.to_string());
                    e.set("reason", "invalid_user");
                }),
            );
        }
        _ => {
            // Allow anyone to delete a project
        }
    }

    let collab_handler = showtimes_db::CollaborationSyncHandler::new(db);
    let collab_info = collab_handler
        .find_by(doc! {
            "projects.project": prj_info.id.to_string(),
            "projects.server": srv.id.to_string()
        })
        .await?;

    if let Some(collab_info) = collab_info {
        let mut collab_info = collab_info;
        // Remove ourselves from the collab
        let collab_project = collab_info.get_and_remove(prj_info.id);

        // Check if actually removed
        if let Some(collab_project) = collab_project {
            // If only 1 or zero, delete this link
            if collab_info.length() < 2 {
                // Delete from DB
                collab_handler.delete(&collab_info).await?;

                // Delete from search engine
                let collab_search =
                    showtimes_search::models::ServerCollabSync::from(collab_info.clone());
                collab_search.delete_document(meili).await?;

                // Delete from search engine
                let collab_clone = collab_info.clone();
                let meili_clone = meili.clone();
                let task_search = tokio::task::spawn(async move {
                    let collab_search =
                        showtimes_search::models::ServerCollabSync::from(collab_clone);
                    collab_search.delete_document(&meili_clone).await
                });
                // Create task events
                let task_events = ctx
                    .data_unchecked::<showtimes_events::SharedSHClickHouse>()
                    .create_event_async(
                        showtimes_events::m::EventKind::CollaborationDeleted,
                        showtimes_events::m::CollabDeletedEvent::new(
                            collab_info.id,
                            &collab_project,
                            true,
                        ),
                        if user.kind == UserKind::Owner {
                            None
                        } else {
                            Some(user.id.to_string())
                        },
                    );

                execute_search_events(task_search, task_events).await?;
            } else {
                // Save the collab
                collab_handler.save(&mut collab_info, None).await?;

                // Update search engine
                let collab_clone = collab_info.clone();
                let meili_clone = meili.clone();
                let task_search = tokio::task::spawn(async move {
                    let collab_search =
                        showtimes_search::models::ServerCollabSync::from(collab_clone);
                    collab_search.update_document(&meili_clone).await
                });
                // Create task events
                let task_events = ctx
                    .data_unchecked::<showtimes_events::SharedSHClickHouse>()
                    .create_event_async(
                        showtimes_events::m::EventKind::CollaborationDeleted,
                        showtimes_events::m::CollabDeletedEvent::new(
                            collab_info.id,
                            &collab_project,
                            false,
                        ),
                        if user.kind == UserKind::Owner {
                            None
                        } else {
                            Some(user.id.to_string())
                        },
                    );

                execute_search_events(task_search, task_events).await?;
            }
        }
    }

    let collab_invite_handler = showtimes_db::CollaborationInviteHandler::new(db);
    let collab_invite_info = collab_invite_handler
        .find_all_by(doc! {
            "$or": [
                {
                    "source.project": prj_info.id.to_string(),
                    "source.server": srv.id.to_string()
                },
                {
                    "target.project": prj_info.id.to_string(),
                }
            ]
        })
        .await?;

    let all_ids = collab_invite_info
        .iter()
        .map(|c| c.id.to_string())
        .collect::<Vec<String>>();

    if !all_ids.is_empty() {
        // Delete from DB
        collab_invite_handler
            .delete_by(doc! {
                "id": {
                    "$in": all_ids.clone()
                }
            })
            .await?;

        // Delete from search engine
        let index_invite = meili.index(showtimes_search::models::ServerCollabInvite::index_name());

        let meili_clone = meili.clone();
        let task_search = tokio::task::spawn(async move {
            match index_invite.delete_documents(&all_ids).await {
                Ok(task_del) => {
                    match task_del.wait_for_completion(&meili_clone, None, None).await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    }
                }
                Err(e) => Err(e),
            }
        });

        let deleted_events: Vec<showtimes_events::m::CollabRetractedEvent> = collab_invite_info
            .iter()
            .map(|collab| showtimes_events::m::CollabRetractedEvent::new(collab.id))
            .collect();

        // Create task events
        let task_events = ctx
            .data_unchecked::<showtimes_events::SharedSHClickHouse>()
            .create_event_many_async(
                showtimes_events::m::EventKind::CollaborationRetracted,
                deleted_events,
                if user.kind == UserKind::Owner {
                    None
                } else {
                    Some(user.id.to_string())
                },
            );

        execute_search_events(task_search, task_events).await?;
    }

    // Delete poster
    let poster_info = &prj_info.poster.image;
    if poster_info.kind == showtimes_fs::FsFileKind::Images.as_path_name() {
        storages
            .file_delete(
                poster_info.key.clone(),
                &poster_info.filename,
                poster_info.parent.as_deref(),
                Some(showtimes_fs::FsFileKind::Images),
            )
            .await?;
    }

    // Delete project
    let prj_handler = ProjectHandler::new(db);
    prj_handler.delete(&prj_info).await?;

    // Delete from search engine
    let prj_clone = prj_info.clone();
    let meili_clone = meili.clone();
    let task_search = tokio::task::spawn(async move {
        let project_search = showtimes_search::models::Project::from(prj_clone);
        project_search.delete_document(&meili_clone).await
    });
    // Create task events
    let task_events = ctx
        .data_unchecked::<showtimes_events::SharedSHClickHouse>()
        .create_event_async(
            showtimes_events::m::EventKind::ProjectDeleted,
            showtimes_events::m::ProjectCreatedEvent::from(&prj_info),
            if user.kind == UserKind::Owner {
                None
            } else {
                Some(user.id.to_string())
            },
        );

    execute_search_events(task_search, task_events).await?;

    Ok(OkResponse::ok("Project deleted"))
}

async fn sync_projects_collaborations(
    ctx: &async_graphql::Context<'_>,
    project: &showtimes_db::m::Project,
    user: &showtimes_db::m::User,
    changed_episodes: ChangedEpisodes,
) -> async_graphql::Result<()> {
    let db = ctx.data_unchecked::<DatabaseShared>();

    // Find all collabs
    let collab_loader = ctx.data_unchecked::<DataLoader<ServerSyncLoader>>();
    let collab_res = collab_loader
        .load_one(ServerSyncIds::new(project.creator, project.id))
        .await?;

    if let Some(collab_info) = collab_res {
        // Get non-current projects
        let other_projects = collab_info
            .projects
            .iter()
            .filter(|p| p.project != project.id && p.server != project.creator)
            .map(|p| p.project)
            .collect::<Vec<showtimes_shared::ulid::Ulid>>();

        if other_projects.is_empty() {
            return Ok(());
        }

        let projects_loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();
        let mut all_projects = projects_loader.load_many(other_projects).await?;
        let prj_handler = ProjectHandler::new(db);

        // Update the roles, status, and assignees
        let mut o_project_search = vec![];
        let mut project_update_events = vec![];
        let mut project_episode_events = vec![];
        for o_project in all_projects.values_mut() {
            let mut before_project = showtimes_events::m::ProjectUpdatedDataEvent::default();
            let mut after_project = showtimes_events::m::ProjectUpdatedDataEvent::default();

            let mut roles_updated = false;
            if !o_project.compare_roles(&project) {
                // When roles update, assignees are also updated
                before_project.set_roles(&o_project.roles);
                before_project.set_assignees(&o_project.assignees);
                roles_updated = true;
            }

            let mut asignees_updated = false;
            if !roles_updated && !o_project.compare_assignees(&project) {
                before_project.set_assignees(&o_project.assignees);
                asignees_updated = true;
            }

            o_project.roles = project.roles.clone();
            o_project.progress = project.progress.clone();
            o_project.propagate_roles();

            if roles_updated {
                after_project.set_roles(&o_project.roles);
            } else {
                before_project.clear_roles();
            }
            if asignees_updated || roles_updated {
                after_project.set_assignees(&o_project.assignees);
            } else {
                before_project.clear_assignees();
            }

            if before_project.has_changes() || after_project.has_changes() {
                project_update_events.push(showtimes_events::m::ProjectUpdatedEvent::new(
                    o_project.id,
                    before_project,
                    after_project,
                ))
            }

            // Update episodes
            let mut any_episode_changes = false;
            if !changed_episodes.is_empty() {
                for (episode, changes) in &changed_episodes {
                    let find_episode = o_project.progress.iter_mut().find(|e| e.number == *episode);
                    if let Some(episode_info) = find_episode {
                        let mut episode_event =
                            showtimes_events::m::ProjectEpisodeUpdatedEvent::new(
                                o_project.id,
                                episode_info.number,
                                changes.silent,
                            );

                        if let Some(finished) = changes.finished {
                            episode_event.set_finished(finished);
                            episode_info.set_finished(finished);
                        }

                        for status in &changes.after {
                            if let Some(role) = episode_info
                                .statuses
                                .iter_mut()
                                .find(|r| r.key() == status.key())
                            {
                                episode_event.push_after(role);
                                role.set_finished(status.finished());
                            } else {
                                let new_role = showtimes_db::m::RoleStatus::new(
                                    status.key().to_string(),
                                    status.finished(),
                                )?;
                                episode_event.push_after(&new_role);
                                episode_info.statuses.push(new_role);
                            }
                        }

                        for before in &changes.before {
                            if let Some(role) = episode_info
                                .statuses
                                .iter()
                                .find(|r| r.key() == before.key())
                            {
                                episode_event.push_before(role);
                            }
                        }

                        if episode_event.has_changes() {
                            project_episode_events.push(episode_event);
                            any_episode_changes = true;
                        }
                    }
                }
            }

            // Save
            if any_episode_changes || roles_updated || asignees_updated {
                prj_handler.save(o_project, None).await?;
                o_project_search.push(showtimes_search::models::Project::from(o_project.clone()));
            }
        }

        // Tasks manager
        let mut all_tasks_manager = vec![];

        // Create project update events
        if !project_update_events.is_empty() {
            let event_ch = ctx
                .data_unchecked::<showtimes_events::SharedSHClickHouse>()
                .clone();
            let user = user.clone();
            let task_events_prj: tokio::task::JoinHandle<Result<(), ProjectEventsError>> =
                tokio::task::spawn(async move {
                    event_ch
                        .create_event_many(
                            showtimes_events::m::EventKind::ProjectUpdated,
                            project_update_events,
                            if user.kind == UserKind::Owner {
                                None
                            } else {
                                Some(user.id.to_string())
                            },
                        )
                        .await
                        .map_err(|e| e.into())
                });

            all_tasks_manager.push(task_events_prj);
        }

        // Create the episodes update event
        if !project_episode_events.is_empty() {
            let event_ch = ctx
                .data_unchecked::<showtimes_events::SharedSHClickHouse>()
                .clone();
            let user = user.clone();
            let task_events_prj: tokio::task::JoinHandle<Result<(), ProjectEventsError>> =
                tokio::task::spawn(async move {
                    event_ch
                        .create_event_many(
                            showtimes_events::m::EventKind::ProjectEpisodes,
                            project_episode_events,
                            if user.kind == UserKind::Owner {
                                None
                            } else {
                                Some(user.id.to_string())
                            },
                        )
                        .await
                        .map_err(|e| e.into())
                });

            all_tasks_manager.push(task_events_prj);
        }

        // Save to search index
        if !o_project_search.is_empty() {
            let meili = ctx.data_unchecked::<SearchClientShared>().clone();
            let o_project_index = meili.index(showtimes_search::models::Project::index_name());
            let task_search = tokio::task::spawn(async move {
                match o_project_index
                    .add_or_update(
                        &o_project_search,
                        Some(showtimes_search::models::Project::primary_key()),
                    )
                    .await
                {
                    Ok(o_project_task) => {
                        match o_project_task.wait_for_completion(&meili, None, None).await {
                            Ok(_) => Ok(()),
                            Err(e) => Err(e.into()),
                        }
                    }
                    Err(e) => Err(e.into()),
                }
            });

            all_tasks_manager.push(task_search);
        }

        // Execute all tasks
        for await_res in futures::future::join_all(all_tasks_manager).await {
            await_res??;
        }
    }

    Ok(())
}

pub async fn mutate_projects_update(
    ctx: &async_graphql::Context<'_>,
    user: showtimes_db::m::User,
    id: UlidGQL,
    input: ProjectUpdateInputGQL,
) -> async_graphql::Result<ProjectGQL> {
    if !input.is_any_set() {
        return Err(Error::new("No fields to update").extend_with(|_, e| {
            e.set("reason", "no_fields");
        }));
    }

    let prj_loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();
    let usr_loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();
    let db = ctx.data_unchecked::<DatabaseShared>();
    let storages = ctx.data_unchecked::<Arc<FsPool>>();
    let meili = ctx.data_unchecked::<SearchClientShared>();

    // Fetch project
    let prj_info = prj_loader.load_one(ProjectDataLoaderKey::Id(*id)).await?;

    if prj_info.is_none() {
        return Err(Error::new("Project not found").extend_with(|_, e| {
            e.set("id", id.to_string());
            e.set("reason", "invalid_project");
        }));
    }

    let mut prj_info = prj_info.unwrap();
    let prj_id = prj_info.id;

    // Check perms
    check_permissions(ctx, prj_info.creator, &user, Some(prj_info.id)).await?;

    let mut before_project = showtimes_events::m::ProjectUpdatedDataEvent::default();
    let mut after_project = showtimes_events::m::ProjectUpdatedDataEvent::default();
    // Update title and aliases
    if let Some(title) = input.title {
        before_project.set_title(&prj_info.title);
        prj_info.title = title;
        after_project.set_title(&prj_info.title);
    }

    if let Some(aliases) = input.aliases {
        before_project.set_aliases(&prj_info.aliases);
        prj_info.aliases = aliases;
        after_project.set_aliases(&prj_info.aliases);
    }

    // Update roles
    if let Some(roles_update) = input.roles {
        before_project.set_roles(&prj_info.roles);

        let mut any_role_changes = false;

        for role in &roles_update {
            match role.action {
                ProjectRoleUpdateAction::Update => {
                    let find_role = prj_info.roles.iter_mut().find(|r| r.key() == role.role.key);
                    if let Some(role_info) = find_role {
                        role_info.set_name(role.role.name.clone());
                        any_role_changes = true;
                    }
                }
                ProjectRoleUpdateAction::Add => {
                    let new_role =
                        showtimes_db::m::Role::new(role.role.key.clone(), role.role.name.clone())?;
                    let mut ordered_roles = prj_info.roles.clone();
                    ordered_roles.sort_by_key(|a| a.order());
                    let last_order = ordered_roles.last().map(|r| r.order()).unwrap_or(0);
                    prj_info.roles.push(new_role.with_order(last_order + 1));
                    any_role_changes = true;
                }
                ProjectRoleUpdateAction::Remove => {
                    let role_count = prj_info.roles.len();
                    prj_info.roles.retain(|r| r.key() != role.role.key);
                    any_role_changes = role_count != prj_info.roles.len();
                }
            }
        }

        if any_role_changes {
            prj_info.propagate_roles();
            after_project.set_roles(&prj_info.roles);
        } else {
            before_project.clear_roles();
        }
    }

    // Update assignees
    if let Some(assignees_update) = input.assignees {
        before_project.set_assignees(&prj_info.assignees);
        let mut any_assignee_changes = false;

        for assignee in &assignees_update {
            // Depending on the ID, remove or add
            let find_role = prj_info
                .assignees
                .iter_mut()
                .find(|a| a.key() == assignee.role);

            if let Some(role_info) = find_role {
                match &assignee.id {
                    Some(id) => {
                        let user_info = usr_loader.load_one(**id).await?;
                        if user_info.is_none() {
                            return Err(Error::new("User not found").extend_with(|_, e| {
                                e.set("id", id.to_string());
                                e.set("role", assignee.role.clone());
                                e.set("action", "update");
                            }));
                        }

                        let user_info = user_info.unwrap();
                        role_info.set_actor(Some(user_info.id));
                    }
                    None => {
                        role_info.set_actor(None);
                    }
                }
                any_assignee_changes = true;
            }
        }

        // Propagate the changes to assignees and roles
        prj_info.propagate_roles();

        if any_assignee_changes {
            after_project.set_assignees(&prj_info.assignees);
        } else {
            before_project.clear_assignees();
        }
    }

    if input.sync_metadata {
        let find_providers = prj_info
            .integrations
            .iter()
            .find(|i| i.kind().is_provider());
        let first_date = prj_info
            .progress
            .first()
            .and_then(|p| p.aired)
            .map(DateTimeGQL::from);

        if let Some(provider) = find_providers {
            let metadata_res = match provider.kind() {
                showtimes_db::m::IntegrationType::ProviderAnilist => {
                    let in_metadata = ProjectCreateMetadataInputGQL {
                        id: provider.id().to_string(),
                        episode: Some(prj_info.progress.len() as i32),
                        start_date: first_date,
                        kind: ExternalSearchSource::Anilist,
                    };

                    fetch_metadata_via_anilist(ctx, &in_metadata).await?
                }
                showtimes_db::m::IntegrationType::ProviderVndb => {
                    let in_metadata = ProjectCreateMetadataInputGQL {
                        id: provider.id().to_string(),
                        episode: Some(prj_info.progress.len() as i32),
                        start_date: first_date,
                        kind: ExternalSearchSource::Vndb,
                    };

                    fetch_metadata_via_vndb(ctx, &in_metadata).await?
                }
                showtimes_db::m::IntegrationType::ProviderTmdb => {
                    let in_metadata = ProjectCreateMetadataInputGQL {
                        id: provider.id().to_string(),
                        episode: Some(prj_info.progress.len() as i32),
                        start_date: first_date,
                        kind: ExternalSearchSource::TMDb,
                    };

                    fetch_metadata_via_tmdb(ctx, &in_metadata).await?
                }
                _ => {
                    return Err(Error::new(format!(
                        "Provider `{}` not supported for metadata sync",
                        provider.kind()
                    ))
                    .extend_with(|_, e| {
                        e.set("provider", provider.kind().to_string());
                        e.set("id", provider.id());
                    }));
                }
            };

            // Update the metadata
            let mut added_episodes: Vec<showtimes_events::m::ProjectUpdatedEpisodeDataEvent> =
                vec![];
            for episode in &metadata_res.progress {
                let find_episode = prj_info.find_episode_mut(episode.number as u64);
                match find_episode {
                    Some(db_ep) => {
                        let mut aired_before =
                            showtimes_events::m::ProjectUpdatedEpisodeDataEvent::updated(
                                episode.number as u64,
                            );
                        if let Some(aired_at) = episode.aired_at {
                            aired_before.set_aired(aired_at.timestamp());
                        }
                        before_project.add_progress(aired_before);
                        db_ep.set_aired(episode.aired_at);
                        let mut aired_after =
                            showtimes_events::m::ProjectUpdatedEpisodeDataEvent::updated(
                                episode.number as u64,
                            );
                        if let Some(aired_at) = episode.aired_at {
                            aired_after.set_aired(aired_at.timestamp());
                        }
                        after_project.add_progress(aired_after);
                    }
                    None => match episode.aired_at {
                        Some(aired_at) => {
                            prj_info.add_episode_with_number_and_airing(
                                episode.number as u64,
                                aired_at,
                            );
                            let mut ep_events =
                                showtimes_events::m::ProjectUpdatedEpisodeDataEvent::added(
                                    episode.number as u64,
                                );
                            ep_events.set_aired(aired_at.timestamp());
                            added_episodes.push(ep_events);
                        }
                        None => {
                            prj_info.add_episode_with_number(episode.number as u64);
                            added_episodes.push(
                                showtimes_events::m::ProjectUpdatedEpisodeDataEvent::added(
                                    episode.number as u64,
                                ),
                            );
                        }
                    },
                }
            }

            for added_ep in added_episodes {
                after_project.add_progress(added_ep);
            }

            // Reverse side, check if we need to remove episodes
            let mut to_be_removed: Vec<u64> = vec![];
            for episode in &prj_info.progress {
                if episode.is_progressing() || episode.is_finished() {
                    continue;
                }
                let find_episode = metadata_res
                    .progress
                    .iter()
                    .find(|e| (e.number as u64) == episode.number);
                if find_episode.is_none() {
                    to_be_removed.push(episode.number);
                }
            }

            for remove_ep in to_be_removed {
                prj_info.remove_episode(remove_ep);
                after_project.add_progress(
                    showtimes_events::m::ProjectUpdatedEpisodeDataEvent::removed(remove_ep),
                );
            }
        }
    }

    // Update progress
    let mut progress_event: Vec<showtimes_events::m::ProjectEpisodeUpdatedEvent> = vec![];
    let mut changed_episodes: ChangedEpisodes = ChangedEpisodes::new();

    if let Some(progress) = input.progress {
        for episode in &progress {
            if !episode.is_any_set() {
                continue;
            }

            let find_episode = prj_info.find_episode_mut(episode.number);
            if let Some(db_ep) = find_episode {
                let mut ep_event = showtimes_events::m::ProjectEpisodeUpdatedEvent::new(
                    prj_id,
                    db_ep.number,
                    episode.silent,
                );

                let mut before_episode =
                    showtimes_events::m::ProjectUpdatedEpisodeDataEvent::updated(db_ep.number);
                let mut after_episode = before_episode.clone();

                let aired_at = episode.aired.clone().map(|a| *a);
                if let Some(original) = db_ep.aired {
                    before_episode.set_aired(original.timestamp());
                }
                db_ep.set_aired(aired_at);
                if let Some(after) = db_ep.aired {
                    after_episode.set_aired(after.timestamp());
                }

                if let Some(true) = episode.unset_delay {
                    if let Some(delay) = &db_ep.delay_reason {
                        before_episode.set_delay_reason(delay);
                    }
                    db_ep.clear_delay_reason();
                } else if let Some(delay) = &episode.delay_reason {
                    if let Some(original) = &db_ep.delay_reason {
                        before_episode.set_delay_reason(original);
                    }
                    db_ep.set_delay_reason(delay);
                    after_episode.set_delay_reason(delay);
                }

                let mut tiny_changes = TinyEpisodeChanges::default();
                tiny_changes.silent = episode.silent;

                if let Some(finished) = episode.finished {
                    db_ep.set_finished(finished);
                    tiny_changes.finished = Some(finished);
                    ep_event.set_finished(finished);
                }

                if let Some(statuses) = &episode.statuses {
                    for status in statuses {
                        let find_status =
                            db_ep.statuses.iter_mut().find(|s| s.key() == status.role);
                        if let Some(status_info) = find_status {
                            tiny_changes.before.push(status_info.clone());
                            ep_event.push_before(&status_info);
                            status_info.set_finished(status.finished);
                            ep_event.push_after(&status_info);
                            tiny_changes.after.push(status_info.clone());
                        }
                    }
                }

                if ep_event.has_changes() {
                    progress_event.push(ep_event);
                }

                if tiny_changes.changed() {
                    changed_episodes.insert(db_ep.number, tiny_changes);
                }

                if episode.is_any_set_except_status() {
                    before_project.add_progress(before_episode);
                    after_project.add_progress(after_episode);
                }
            }
        }
    }

    // Update poster
    if let Some(poster_upload) = input.poster {
        let info_up = poster_upload.value(ctx)?;
        let mut file_target = tokio::fs::File::from_std(info_up.content);

        // Get format
        let format = crate::image::detect_upload_data(&mut file_target).await?;
        // Seek back to the start of the file
        file_target.seek(std::io::SeekFrom::Start(0)).await?;

        let filename = format!("cover.{}", format.as_extension());

        storages
            .file_stream_upload(
                prj_info.id,
                &filename,
                file_target,
                Some(&prj_info.creator.to_string()),
                Some(showtimes_fs::FsFileKind::Images),
            )
            .await?;

        let image_meta = showtimes_db::m::ImageMetadata::new(
            showtimes_fs::FsFileKind::Images.as_path_name(),
            prj_info.id,
            &filename,
            format.as_extension(),
            Some(prj_info.creator.to_string()),
        );

        before_project.set_poster_image(&prj_info.poster.image);
        prj_info.poster.image = image_meta;
        after_project.set_poster_image(&prj_info.poster.image);
    }

    if let Some(poster_color) = input.poster_color {
        prj_info.poster.color = Some(poster_color);
    }

    let event_ch = ctx
        .data_unchecked::<showtimes_events::SharedSHClickHouse>()
        .clone();

    let task_events_prj: tokio::task::JoinHandle<Result<(), ProjectEventsError>> =
        tokio::task::spawn(async move {
            event_ch
                .create_event(
                    showtimes_events::m::EventKind::ProjectUpdated,
                    showtimes_events::m::ProjectUpdatedEvent::new(
                        prj_info.id,
                        before_project,
                        after_project,
                    ),
                    if user.kind == UserKind::Owner {
                        None
                    } else {
                        Some(user.id.to_string())
                    },
                )
                .await
                .map_err(|e| e.into())
        });

    let mut all_events_manager = vec![];
    all_events_manager.push(task_events_prj);

    if !progress_event.is_empty() {
        let event_ch = ctx
            .data_unchecked::<showtimes_events::SharedSHClickHouse>()
            .clone();
        let task_events_eps = tokio::task::spawn(async move {
            event_ch
                .create_event_many(
                    showtimes_events::m::EventKind::ProjectEpisodes,
                    progress_event,
                    if user.kind == UserKind::Owner {
                        None
                    } else {
                        Some(user.id.to_string())
                    },
                )
                .await
                .map_err(|e| e.into())
        });

        all_events_manager.push(task_events_eps);
    }

    // Save the project
    let prj_gql =
        make_and_update_project_events(db, meili, &mut prj_info, &mut all_events_manager).await?;
    // Sync the collaborations
    sync_projects_collaborations(ctx, &prj_info, &user, changed_episodes).await?;

    Ok(prj_gql)
}

pub async fn mutate_projects_episode_add_auto(
    ctx: &async_graphql::Context<'_>,
    user: showtimes_db::m::User,
    id: UlidGQL,
    count: u64,
) -> async_graphql::Result<ProjectGQL> {
    let prj_loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();
    let db = ctx.data_unchecked::<DatabaseShared>();
    let meili = ctx.data_unchecked::<SearchClientShared>();

    // Fetch project
    let prj_info = prj_loader.load_one(ProjectDataLoaderKey::Id(*id)).await?;

    if prj_info.is_none() {
        return Err(Error::new("Project not found").extend_with(|_, e| {
            e.set("id", id.to_string());
            e.set("reason", "invalid_project");
        }));
    }

    let mut prj_info = prj_info.unwrap();

    // Check perms
    check_permissions(ctx, prj_info.creator, &user, Some(prj_info.id)).await?;

    // Add episodes from the last episode
    let mut sorted_episodes = prj_info.progress.clone();
    sorted_episodes.sort();
    let last_episode = sorted_episodes.last().unwrap();
    let last_air_date = last_episode.aired;
    let new_episodes = ((last_episode.number + 1)..=(last_episode.number + count))
        .enumerate()
        .map(|(idx, n)| {
            let next_air_date =
                last_air_date.map(|d| d + chrono::Duration::weeks((idx + 1) as i64));

            let mut ep =
                showtimes_db::m::EpisodeProgress::new_with_roles(n, false, &prj_info.roles);
            ep.set_aired(next_air_date);
            ep
        })
        .collect::<Vec<showtimes_db::m::EpisodeProgress>>();

    // Extend, sort, then replace
    sorted_episodes.extend(new_episodes);
    sorted_episodes.sort();
    prj_info.progress = sorted_episodes;

    // Save the project
    let prj_gql = make_and_update_project(db, meili, &mut prj_info).await?;
    // Sync the collaborations
    sync_projects_collaborations(ctx, &prj_info, &user, ChangedEpisodes::new()).await?;

    Ok(prj_gql)
}

pub async fn mutate_projects_episode_add_manual(
    ctx: &async_graphql::Context<'_>,
    user: showtimes_db::m::User,
    id: UlidGQL,
    episodes: &[ProgressCreateInputGQL],
) -> async_graphql::Result<ProjectGQL> {
    let prj_loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();
    let db = ctx.data_unchecked::<DatabaseShared>();
    let meili = ctx.data_unchecked::<SearchClientShared>();

    // Fetch project
    let prj_info = prj_loader.load_one(ProjectDataLoaderKey::Id(*id)).await?;

    if prj_info.is_none() {
        return Err(Error::new("Project not found").extend_with(|_, e| {
            e.set("id", id.to_string());
            e.set("reason", "invalid_project");
        }));
    }

    let mut prj_info = prj_info.unwrap();

    // Check perms
    check_permissions(ctx, prj_info.creator, &user, Some(prj_info.id)).await?;

    // Add episodes
    for episode in episodes {
        let exist_mut = prj_info.find_episode_mut(episode.number);
        if let Some(repl_mut) = exist_mut {
            if let Some(aired) = &episode.aired {
                repl_mut.set_aired(Some(**aired));
            }
        } else {
            match &episode.aired {
                Some(aired) => {
                    prj_info.add_episode_with_number_and_airing(episode.number, **aired);
                }
                None => {
                    prj_info.add_episode_with_number(episode.number);
                }
            }
        }
    }

    // Sort the episodes
    prj_info.sort_progress();

    // Save the project
    let prj_gql = make_and_update_project(db, meili, &mut prj_info).await?;
    // Sync the collaborations
    sync_projects_collaborations(ctx, &prj_info, &user, ChangedEpisodes::new()).await?;

    Ok(prj_gql)
}

pub async fn mutate_projects_episode_remove(
    ctx: &async_graphql::Context<'_>,
    user: showtimes_db::m::User,
    id: UlidGQL,
    episodes: &[u64],
) -> async_graphql::Result<ProjectGQL> {
    let prj_loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();
    let db = ctx.data_unchecked::<DatabaseShared>();
    let meili = ctx.data_unchecked::<SearchClientShared>();

    // Fetch project
    let prj_info = prj_loader.load_one(ProjectDataLoaderKey::Id(*id)).await?;

    if prj_info.is_none() {
        return Err(Error::new("Project not found").extend_with(|_, e| {
            e.set("id", id.to_string());
            e.set("reason", "invalid_project");
        }));
    }

    let mut prj_info = prj_info.unwrap();

    // Check perms
    check_permissions(ctx, prj_info.creator, &user, Some(prj_info.id)).await?;

    // Remove episodes marked
    prj_info
        .progress
        .retain(|ep| !episodes.contains(&ep.number));

    // Save the project
    let prj_gql = make_and_update_project(db, meili, &mut prj_info).await?;
    // Sync the collaborations
    sync_projects_collaborations(ctx, &prj_info, &user, ChangedEpisodes::new()).await?;

    Ok(prj_gql)
}
