use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use async_graphql::{dataloader::DataLoader, CustomValidator, Enum, InputObject, Upload};
use chrono::TimeZone;
use showtimes_db::{m::UserKind, mongodb::bson::doc, DatabaseShared, ProjectHandler};
use showtimes_fs::FsPool;
use showtimes_metadata::m::AnilistMediaFormat;
use showtimes_search::SearchClientShared;
use tokio::{io::AsyncSeekExt, sync::Mutex};

use showtimes_gql_common::{
    data_loader::{
        ProjectDataLoader, ServerDataLoader, ServerSyncIds, ServerSyncLoader, UserDataLoader,
    },
    errors::GQLError,
    DateTimeGQL, GQLErrorCode, GQLErrorExt, OkResponse, UlidGQL,
};
use showtimes_gql_models::{
    projects::{ProjectGQL, ProjectStatusGQL},
    search::ExternalSearchSource,
};

use crate::execute_search_events;

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
    /// Do a silent update, this will not broadcast the event for status change.
    ///
    /// This will be overridden when it's updating other synchronized project and
    /// that project status is currently set to `ARCHIVED`.
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
    /// The status of the project.
    ///
    /// The only thing that can be modified when project is "ARCHIVED" is this field.
    ///
    /// When you're providing this, all other fields will be ignored.
    status: Option<ProjectStatusGQL>,
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
            || self.status.is_some()
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
    let id_fetch =
        input
            .id
            .parse::<i32>()
            .extend_error(GQLErrorCode::MetadataAnilistInvalidId, |e| {
                e.set("id", input.id.clone());
                e.set("source", "anilist");
            })?;

    let anilist_loader = ctx.data_unchecked::<Arc<Mutex<showtimes_metadata::AnilistProvider>>>();
    let mut anilist = anilist_loader.lock().await;

    let anilist_info = anilist.get_media(id_fetch).await.map_err(|err| {
        GQLError::new(err.to_string(), GQLErrorCode::MetadataAnilistRequestError).extend(|e| {
            e.set("id", id_fetch);
            e.set("source", "anilist");
        })
    })?;

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
        GQLError::new(
            "No title found from metadata".to_string(),
            GQLErrorCode::MetadataError,
        )
        .extend(|e| {
            e.set("id", id_fetch);
            e.set("source", "anilist");
        })
    })?;

    let mut merged_episodes: Vec<ExternalMediaFetchProgressResult> = vec![];
    let mut continue_fetch = true;
    let mut current_page = 1;
    while continue_fetch {
        let air_sched = anilist
            .get_airing_schedules(id_fetch, Some(current_page))
            .await
            .map_err(|err| {
                GQLError::new(err.to_string(), GQLErrorCode::MetadataAnilistRequestError).extend(
                    |e| {
                        e.set("id", id_fetch);
                        e.set("page", current_page);
                        e.set("when", "airing_schedules");
                        e.set("source", "anilist");
                    },
                )
            })?;
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
                return GQLError::new(
                    "No episodes found from metadata".to_string(),
                    GQLErrorCode::MetadataNoEpisodesFound,
                )
                .extend(|e| {
                    e.set("id", id_fetch);
                    e.set("source", "anilist");
                })
                .into();
            }

            episode_count
        }
        showtimes_metadata::m::AnilistMediaType::Manga => anilist_info
            .chapters
            .unwrap_or_else(|| input.episode.unwrap_or(0)),
    };

    let start_time = match (anilist_info.start_date, input.start_date) {
        (_, Some(start_date)) => *start_date,
        (Some(fuzzy_start), _) => fuzzy_start.into_chrono().ok_or_else(|| {
            GQLError::new(
                "Invalid fuzzy date from Anilist, please provide override".to_string(),
                GQLErrorCode::MetadataUnableToParseDate,
            )
            .extend(|e| {
                e.set("id", id_fetch);
                e.set("date", fuzzy_start.to_string());
                e.set("source", "anilist");
            })
        })?,
        _ => {
            return GQLError::new(
                "No start date found from metadata".to_string(),
                GQLErrorCode::MetadataNoStartDate,
            )
            .extend(|e| {
                e.set("id", id_fetch);
                e.set("source", "anilist");
            })
            .into();
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
                return GQLError::new(
                    "No episodes found from metadata".to_string(),
                    GQLErrorCode::MetadataNoEpisodesFound,
                )
                .extend(|e| {
                    e.set("id", id_fetch);
                    e.set("source", "anilist");
                })
                .into();
            }

            // Check the episode range if all exists
            let first_ep = merged_episodes[0].clone();
            let last_ep = merged_episodes
                .clone()
                .last()
                .expect("No last episode even though it should exist")
                .clone();
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
        return GQLError::new(
            "VNDB ID must not start with 'v'",
            GQLErrorCode::MetadataVNDBInvalidId,
        )
        .extend(|e| {
            e.set("id", input_id);
            e.set("source", "vndb");
        })
        .into();
    }
    let id_test = input_id.trim_start_matches('v');
    if id_test.parse::<u64>().is_err() {
        return GQLError::new(
            "VNDB ID must have a number only after 'v'",
            GQLErrorCode::MetadataVNDBInvalidId,
        )
        .extend(|e| {
            e.set("id", input_id);
            e.set("source", "vndb");
        })
        .into();
    }

    let vndb_info = vndb_loader.get(input_id).await.map_err(|err| {
        GQLError::new(err.to_string(), GQLErrorCode::MetadataVNDBRequestError).extend(|e| {
            e.set("id", input_id);
            e.set("source", "vndb");
        })
    })?;

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
        GQLError::new(
            "No title found from metadata".to_string(),
            GQLErrorCode::MetadataError,
        )
        .extend(|e| {
            e.set("id", input_id);
            e.set("source", "vndb");
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
        GQLError::new("Invalid TMDb ID", GQLErrorCode::MetadataTMDbInvalidId).extend(|e| {
            e.set("id", input.id.clone());
            e.set("source", "tmdb");
        })
    })?;
    let tmdb_loader = ctx.data_unchecked::<Arc<showtimes_metadata::TMDbProvider>>();

    let tmdb_info = tmdb_loader
        .get_movie_details(input_id)
        .await
        .map_err(|err| {
            GQLError::new(err.to_string(), GQLErrorCode::MetadataTMDbRequestError).extend(|e| {
                e.set("id", input_id);
                e.set("source", "tmdb");
            })
        })?;

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
        GQLError::new(
            "No title found from metadata".to_string(),
            GQLErrorCode::MetadataError,
        )
        .extend(|e| {
            e.set("id", input_id);
            e.set("source", "vndb");
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

async fn check_permissions(
    ctx: &async_graphql::Context<'_>,
    id: showtimes_shared::ulid::Ulid,
    user: &showtimes_db::m::User,
    project_id: Option<showtimes_shared::ulid::Ulid>,
) -> async_graphql::Result<showtimes_db::m::Server> {
    let srv_loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();

    let srv = srv_loader.load_one(id).await?.ok_or_else(|| {
        GQLError::new("Server not found", GQLErrorCode::ServerNotFound)
            .extend(|e| e.set("id", id.to_string()))
    })?;

    let find_user = srv.owners.iter().find(|o| o.id == user.id);

    match (find_user, user.kind) {
        (Some(user), showtimes_db::m::UserKind::User) => {
            // Check if we are allowed to create a project
            match project_id {
                Some(project_id) => {
                    if user.privilege == showtimes_db::m::UserPrivilege::ProjectManager
                        && !user.has_id(project_id)
                    {
                        GQLError::new(
                            "User not allowed to manage project",
                            GQLErrorCode::UserInsufficientPrivilege,
                        )
                        .extend(|e| {
                            e.set("id", id.to_string());
                            e.set("current", user.privilege.to_string());
                            e.set(
                                "required",
                                showtimes_db::m::UserPrivilege::ProjectManager.to_string(),
                            );
                            e.set("is_in_server", true);
                        })
                        .into()
                    } else {
                        Ok(srv)
                    }
                }
                None => {
                    if user.privilege < showtimes_db::m::UserPrivilege::Manager {
                        GQLError::new(
                            "User not allowed to create/delete projects",
                            GQLErrorCode::UserInsufficientPrivilege,
                        )
                        .extend(|e| {
                            e.set("id", id.to_string());
                            e.set("current", user.privilege.to_string());
                            e.set(
                                "required",
                                showtimes_db::m::UserPrivilege::Manager.to_string(),
                            );
                            e.set("is_in_server", true);
                        })
                        .into()
                    } else {
                        Ok(srv)
                    }
                }
            }
        }
        (None, showtimes_db::m::UserKind::User) => GQLError::new(
            "User not allowed to create projects",
            GQLErrorCode::UserInsufficientPrivilege,
        )
        .extend(|e| {
            e.set("id", id.to_string());
            e.set("is_in_server", false);
        })
        .into(),
        _ => {
            // Allow anyone to create a project
            Ok(srv)
        }
    }
}

async fn fetch_project_collaborators(
    ctx: &async_graphql::Context<'_>,
    project: &showtimes_db::m::Project,
) -> async_graphql::Result<Vec<showtimes_db::m::Project>> {
    let collab_loader = ctx.data_unchecked::<DataLoader<ServerSyncLoader>>();
    let collab_res = collab_loader
        .load_one(ServerSyncIds::new(project.creator, project.id))
        .await?;

    match collab_res {
        Some(collab) => {
            let prj_loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();
            let other_projects: Vec<showtimes_shared::ulid::Ulid> = collab
                .projects
                .iter()
                .filter_map(|p| {
                    if p.project == project.id {
                        None
                    } else {
                        Some(p.project)
                    }
                })
                .collect();

            let loaded_projects = prj_loader.load_many(other_projects).await?;

            // Get values of hashmap
            Ok(loaded_projects.values().cloned().collect())
        }
        None => Ok(vec![]),
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
                        GQLError::new(e_root.to_string(), GQLErrorCode::InvalidRequest).extend(
                            |e| {
                                e.set("server", id.to_string());
                                e.set("index", idx);
                                e.set("key", role.key.clone());
                                e.set("name", role.name.clone());
                            },
                        )
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
                        // TODO: Propagate error properly
                        assignees.push(showtimes_db::m::RoleAssignee::new(
                            role.key(),
                            Some(user_info.id),
                        )?);
                    }
                    None => {
                        // TODO: Propagate error properly
                        assignees.push(showtimes_db::m::RoleAssignee::new(role.key(), None)?);
                    }
                }
            }
            None => {
                // TODO: Propagate error properly
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

    // TODO: Propagate error properly
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
            let info_up = poster.value(ctx).map_err(|err| {
                GQLError::new(
                    format!("Failed to read image upload: {err}"),
                    GQLErrorCode::IOError,
                )
                .extend(|e| {
                    e.set("id", project.id.to_string());
                    e.set("where", "project");
                    e.set("original", format!("{err}"));
                    e.set("original_code", format!("{}", err.kind()));
                })
            })?;
            let mut file_target = tokio::fs::File::from_std(info_up.content);

            // Get format
            let format = showtimes_gql_common::image::detect_upload_data(&mut file_target)
                .await
                .map_err(|err| {
                    GQLError::new(
                        format!("Failed to detect image format: {err}"),
                        GQLErrorCode::IOError,
                    )
                    .extend(|e| {
                        e.set("id", project.id.to_string());
                        e.set("where", "project");
                        e.set("original", format!("{err}"));
                        e.set("original_code", format!("{}", err.kind()));
                    })
                })?;
            // Seek back to the start of the file
            file_target
                .seek(std::io::SeekFrom::Start(0))
                .await
                .map_err(|err| {
                    GQLError::new(
                        format!("Failed to seek to image to start: {err}"),
                        GQLErrorCode::IOError,
                    )
                    .extend(|e| {
                        e.set("id", project.id.to_string());
                        e.set("where", "project");
                        e.set("original", format!("{err}"));
                        e.set("original_code", format!("{}", err.kind()));
                    })
                })?;

            let filename = format!("cover.{}", format.as_extension());

            storages
                .file_stream_upload(
                    project.id,
                    &filename,
                    file_target,
                    Some(&srv.id.to_string()),
                    Some(showtimes_fs::FsFileKind::Images),
                )
                .await
                .map_err(|err| {
                    GQLError::new(
                        format!("Failed to upload image: {err}"),
                        GQLErrorCode::ImageUploadError,
                    )
                    .extend(|e| {
                        e.set("id", project.id.to_string());
                        e.set("where", "project");
                        e.set("original", format!("{err}"));
                    })
                })?;

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
                .await
                .map_err(|err| {
                    GQLError::new(
                        format!("Failed to upload image: {err}"),
                        GQLErrorCode::ImageUploadError,
                    )
                    .extend(|e| {
                        e.set("id", project.id.to_string());
                        e.set("where", "project");
                        e.set("original", format!("{err}"));
                    })
                })?;

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
    });

    // Save the project
    let prj_handler = ProjectHandler::new(db);
    // TODO: Propagate error properly
    prj_handler.save(&mut project, None).await?;

    // Update index
    let prj_search = showtimes_search::models::Project::from(&project);
    let meili_clone = ctx.data_unchecked::<SearchClientShared>().clone();
    let task_search =
        tokio::task::spawn(async move { prj_search.update_document(&meili_clone).await });

    execute_search_events(task_search, task_events).await?;

    let prj_gql = ProjectGQL::from(&project);
    Ok(prj_gql)
}

async fn download_cover(url: &str) -> async_graphql::Result<Vec<u8>> {
    let ua_ver = format!(
        "showtimes-rs-gql/{} (+https://github.com/naoTimesdev/showtimes-rs)",
        env!("CARGO_PKG_VERSION")
    );
    let mut header_maps = reqwest::header::HeaderMap::new();
    header_maps.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_str(&ua_ver).map_err(|_| {
            GQLError::new(
                "Failed to parse user agent for downloading cover",
                GQLErrorCode::MetadataClientError,
            )
            .extend(|f| {
                f.set("url", url);
            })
        })?,
    );

    let client = reqwest::ClientBuilder::new()
        .http2_adaptive_window(true)
        .default_headers(header_maps)
        .use_rustls_tls()
        .build()
        .map_err(|e| {
            GQLError::new(
                "Failed to create request client for downloading cover",
                GQLErrorCode::MetadataClientError,
            )
            .extend(|f| {
                f.set("url", url);
                f.set("original", format!("{}", e));
            })
        })?;

    let resp = client.get(url).send().await.map_err(|e| {
        GQLError::new(
            "Failed to fetch cover from url",
            GQLErrorCode::MetadataClientError,
        )
        .extend(|f| {
            f.set("url", url);
            f.set("original", format!("{}", e));
        })
    })?;

    if !resp.status().is_success() {
        return Err(GQLError::new(
            format!("Failed to download cover: {}", url),
            GQLErrorCode::MetadataPosterError,
        )
        .extend(|f| {
            f.set("url", url);
            f.set("status_code", resp.status().as_u16());
        })
        .into());
    }

    let bytes = resp.bytes().await.map_err(|e| {
        GQLError::new(
            format!("Failed to process cover bytes: {}", url),
            GQLErrorCode::MetadataPosterError,
        )
        .extend(|f| {
            f.set("url", url);
            f.set("original", format!("{}", e));
        })
    })?;
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

    // Fetch project
    let prj_info = prj_loader.load_one(*id).await?.ok_or_else(|| {
        GQLError::new("Project not found", GQLErrorCode::ProjectNotFound)
            .extend(|e| e.set("id", id.to_string()))
    })?;

    let srv = srv_loader
        .load_one(prj_info.creator)
        .await?
        .ok_or_else(|| {
            GQLError::new("Server not found", GQLErrorCode::ServerNotFound)
                .extend(|e| e.set("id", prj_info.creator.to_string()))
        })?;

    let find_user = srv.owners.iter().find(|o| o.id == user.id);
    match (find_user, user.kind) {
        (Some(user), showtimes_db::m::UserKind::User) => {
            // Check if we are allowed to create a project
            if user.privilege < showtimes_db::m::UserPrivilege::Manager {
                return GQLError::new(
                    "User not allowed to delete projects",
                    GQLErrorCode::UserInsufficientPrivilege,
                )
                .extend(|e| {
                    e.set("id", id.to_string());
                    e.set("server", srv.id.to_string());
                    e.set("current", user.privilege.to_string());
                    e.set(
                        "required",
                        showtimes_db::m::UserPrivilege::Manager.to_string(),
                    );
                    e.set("is_in_server", true);
                })
                .into();
            }
        }
        (None, showtimes_db::m::UserKind::User) => {
            return GQLError::new(
                "User not allowed to delete projects",
                GQLErrorCode::UserInsufficientPrivilege,
            )
            .extend(|e| {
                e.set("id", id.to_string());
                e.set("server", srv.id.to_string());
                e.set("is_in_server", false);
            })
            .into();
        }
        _ => {
            // Allow anyone to delete a project
        }
    }

    let collab_handler = showtimes_db::CollaborationSyncHandler::new(db);
    // TODO: Propagate error properly
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
                // TODO: Propagate error properly
                collab_handler.delete(&collab_info).await?;

                // Delete from search engine
                let collab_search =
                    showtimes_search::models::ServerCollabSync::from(collab_info.clone());
                // TODO: Propagate error properly
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
                // TODO: Propagate error properly
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

                // TODO: Propagate error properly
                execute_search_events(task_search, task_events).await?;
            }
        }
    }

    let collab_invite_handler = showtimes_db::CollaborationInviteHandler::new(db);
    // TODO: Propagate error properly
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
        // TODO: Propagate error properly
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
        // TODO: Propagate error properly
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
    // TODO: Propagate error properly
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

/// Update a single project
///
/// This will mutate the project in-place so it can be saved later.
async fn update_single_project(
    ctx: &async_graphql::Context<'_>,
    project: &mut showtimes_db::m::Project,
    input: &ProjectUpdateInputGQL,
    metadata: Option<&ExternalMediaFetchResult>,
    loaded_users: &HashMap<showtimes_shared::ulid::Ulid, showtimes_db::m::User>,
    is_main: bool,
) -> async_graphql::Result<(
    showtimes_events::m::ProjectUpdatedEvent,
    Vec<showtimes_events::m::ProjectEpisodeUpdatedEvent>,
)> {
    let storages = ctx.data_unchecked::<Arc<FsPool>>();
    let prj_id = project.id;

    let mut before_project = showtimes_events::m::ProjectUpdatedDataEvent::default();
    let mut after_project = showtimes_events::m::ProjectUpdatedDataEvent::default();
    if is_main {
        // Update title
        if let Some(title) = &input.title {
            before_project.set_title(&project.title);
            project.title = title.clone();
            after_project.set_title(&project.title);
        }

        // Update aliases
        if let Some(aliases) = &input.aliases {
            before_project.set_aliases(&project.aliases.clone());
            project.aliases = aliases.clone();
            after_project.set_aliases(&project.aliases.clone());
        }
    }

    // Update roles
    if let Some(roles_update) = &input.roles {
        before_project.set_roles(&project.roles);

        let mut any_role_changes = false;

        for role in roles_update {
            match role.action {
                ProjectRoleUpdateAction::Update => {
                    let find_role = project.roles.iter_mut().find(|r| r.key() == role.role.key);
                    if let Some(role_info) = find_role {
                        role_info.set_name(role.role.name.clone());
                        any_role_changes = true;
                    }
                }
                ProjectRoleUpdateAction::Add => {
                    // TODO: Propagate error properly
                    let new_role =
                        showtimes_db::m::Role::new(role.role.key.clone(), role.role.name.clone())?;
                    let mut ordered_roles = project.roles.clone();
                    ordered_roles.sort_by_key(|a| a.order());
                    let last_order = ordered_roles.last().map(|r| r.order()).unwrap_or(0);
                    project.roles.push(new_role.with_order(last_order + 1));
                    any_role_changes = true;
                }
                ProjectRoleUpdateAction::Remove => {
                    let role_count = project.roles.len();
                    project.roles.retain(|r| r.key() != role.role.key);
                    any_role_changes = role_count != project.roles.len();
                }
            }
        }

        if any_role_changes {
            project.propagate_roles();
            after_project.set_roles(&project.roles);
        } else {
            before_project.clear_roles();
        }
    }

    // Update assignees
    if let Some(assignees_update) = &input.assignees {
        before_project.set_assignees(&project.assignees);
        let mut any_assignee_changes = false;

        for assignee in assignees_update {
            // Depending on the ID, remove or add
            let find_role = project
                .assignees
                .iter_mut()
                .find(|a| a.key() == assignee.role);

            if let Some(role_info) = find_role {
                match &assignee.id {
                    Some(id) => {
                        let user_info = loaded_users.get(&**id).ok_or_else(|| {
                            GQLError::new("User not found", GQLErrorCode::UserNotFound).extend(
                                |e| {
                                    e.set("id", id.to_string());
                                    e.set("project", prj_id.to_string());
                                    e.set("role", &assignee.role);
                                    e.set("action", "update");
                                },
                            )
                        })?;

                        role_info.set_actor(Some(user_info.id));
                    }
                    None => {
                        role_info.set_actor(None);
                    }
                }
                any_assignee_changes = true;
            }
        }

        if any_assignee_changes {
            after_project.set_assignees(&project.assignees);
        } else {
            before_project.clear_assignees();
        }
    }

    if let Some(metadata_res) = metadata {
        // Update the metadata
        for episode in &metadata_res.progress {
            let find_episode = project.find_episode_mut(episode.number as u64);
            match find_episode {
                Some(db_ep) => {
                    let mut aired_before =
                        showtimes_events::m::ProjectUpdatedEpisodeDataEvent::updated(
                            episode.number as u64,
                        );
                    if let Some(aired_at) = db_ep.aired {
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
                        project.add_episode_with_number_and_airing(episode.number as u64, aired_at);
                        let mut ep_events =
                            showtimes_events::m::ProjectUpdatedEpisodeDataEvent::added(
                                episode.number as u64,
                            );
                        ep_events.set_aired(aired_at.timestamp());
                        after_project.add_progress(ep_events);
                    }
                    None => {
                        project.add_episode_with_number(episode.number as u64);
                        after_project.add_progress(
                            showtimes_events::m::ProjectUpdatedEpisodeDataEvent::added(
                                episode.number as u64,
                            ),
                        );
                    }
                },
            }
        }

        // Reverse side, check if we need to remove episodes
        let mut to_be_removed: Vec<u64> = vec![];
        for episode in &project.progress {
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
            project.remove_episode(remove_ep);
            before_project.add_progress(
                showtimes_events::m::ProjectUpdatedEpisodeDataEvent::removed(remove_ep),
            );
        }
    }

    // Update progress
    let mut progress_event: Vec<showtimes_events::m::ProjectEpisodeUpdatedEvent> = vec![];

    let is_archived = project.status == showtimes_db::m::ProjectStatus::Archived;

    if let Some(progress) = &input.progress {
        for episode in progress {
            if !episode.is_any_set() {
                continue;
            }

            let find_episode = project.find_episode_mut(episode.number);
            if let Some(db_ep) = find_episode {
                let is_archived_silent = !is_main && is_archived;
                let mut ep_event = showtimes_events::m::ProjectEpisodeUpdatedEvent::new(
                    prj_id,
                    db_ep.number,
                    episode.silent || is_archived_silent,
                );

                let mut before_episode =
                    showtimes_events::m::ProjectUpdatedEpisodeDataEvent::updated(db_ep.number);
                let mut after_episode = before_episode.clone();

                let aired_at = episode.aired.map(|a| *a);
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

                if let Some(finished) = episode.finished {
                    db_ep.set_finished(finished);
                    ep_event.set_finished(finished);
                }

                if let Some(statuses) = &episode.statuses {
                    for status in statuses {
                        let find_status =
                            db_ep.statuses.iter_mut().find(|s| s.key() == status.role);
                        if let Some(status_info) = find_status {
                            ep_event.push_before(status_info);
                            status_info.set_finished(status.finished);
                            ep_event.push_after(status_info);
                        }
                    }
                }

                if ep_event.has_changes() {
                    progress_event.push(ep_event);
                }

                if episode.is_any_set_except_status() {
                    before_project.add_progress(before_episode);
                    after_project.add_progress(after_episode);
                }
            }
        }
    }

    // Update poster
    if is_main {
        if let Some(poster_upload) = input.poster {
            let info_up = poster_upload.value(ctx).map_err(|err| {
                GQLError::new(
                    format!("Failed to read image upload: {err}"),
                    GQLErrorCode::IOError,
                )
                .extend(|e| {
                    e.set("id", prj_id.to_string());
                    e.set("where", "project");
                    e.set("original", format!("{err}"));
                    e.set("original_code", format!("{}", err.kind()));
                })
            })?;
            let mut file_target = tokio::fs::File::from_std(info_up.content);

            // Get format
            let format = showtimes_gql_common::image::detect_upload_data(&mut file_target)
                .await
                .map_err(|err| {
                    GQLError::new(
                        format!("Failed to detect image format: {err}"),
                        GQLErrorCode::IOError,
                    )
                    .extend(|e| {
                        e.set("id", prj_id.to_string());
                        e.set("where", "project");
                        e.set("original", format!("{err}"));
                        e.set("original_code", format!("{}", err.kind()));
                    })
                })?;
            // Seek back to the start of the file
            file_target
                .seek(std::io::SeekFrom::Start(0))
                .await
                .map_err(|err| {
                    GQLError::new(
                        format!("Failed to seek to image to start: {err}"),
                        GQLErrorCode::IOError,
                    )
                    .extend(|e| {
                        e.set("id", prj_id.to_string());
                        e.set("where", "project");
                        e.set("original", format!("{err}"));
                        e.set("original_code", format!("{}", err.kind()));
                    })
                })?;

            let filename = format!("cover.{}", format.as_extension());

            storages
                .file_stream_upload(
                    prj_id,
                    &filename,
                    file_target,
                    Some(&project.creator.to_string()),
                    Some(showtimes_fs::FsFileKind::Images),
                )
                .await
                .map_err(|err| {
                    GQLError::new(
                        format!("Failed to upload image: {err}"),
                        GQLErrorCode::ImageUploadError,
                    )
                    .extend(|e| {
                        e.set("id", prj_id.to_string());
                        e.set("where", "project");
                        e.set("original", format!("{err}"));
                    })
                })?;

            let image_meta = showtimes_db::m::ImageMetadata::new(
                showtimes_fs::FsFileKind::Images.as_path_name(),
                prj_id,
                &filename,
                format.as_extension(),
                Some(project.creator.to_string()),
            );

            before_project.set_poster_image(&project.poster.image);
            project.poster.image = image_meta;
            after_project.set_poster_image(&project.poster.image);
        }

        // Update poster color
        if let Some(poster_color) = input.poster_color {
            project.poster.color = Some(poster_color);
        }
    }

    Ok((
        showtimes_events::m::ProjectUpdatedEvent::new(project.id, before_project, after_project),
        progress_event,
    ))
}

pub async fn mutate_projects_update(
    ctx: &async_graphql::Context<'_>,
    user: showtimes_db::m::User,
    id: UlidGQL,
    input: ProjectUpdateInputGQL,
) -> async_graphql::Result<ProjectGQL> {
    if !input.is_any_set() {
        return GQLError::new("No fields to update", GQLErrorCode::MissingModification).into();
    }

    let prj_loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();
    let usr_loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();
    let db = ctx.data_unchecked::<DatabaseShared>();

    let prj_handler = ProjectHandler::new(db);

    // Fetch project
    let mut prj_info = prj_loader.load_one(*id).await?.ok_or_else(|| {
        GQLError::new("Project not found", GQLErrorCode::ProjectNotFound)
            .extend(|e| e.set("id", id.to_string()))
    })?;

    // Check perms
    check_permissions(ctx, prj_info.creator, &user, Some(prj_info.id)).await?;

    // Check if we can update project
    match (prj_info.status, input.status) {
        (showtimes_db::m::ProjectStatus::Archived, None) => {
            // Missing status update and in archived mode, error!
            return GQLError::new("Project is archived", GQLErrorCode::ProjectArchived)
                .extend(|e| e.set("id", id.to_string()))
                .into();
        }
        (showtimes_db::m::ProjectStatus::Archived, Some(new_status)) => {
            if new_status == ProjectStatusGQL::Archived {
                return GQLError::new("Project is archived", GQLErrorCode::ProjectArchived)
                    .extend(|e| e.set("id", id.to_string()))
                    .into();
            }

            let mut before_update = showtimes_events::m::ProjectUpdatedDataEvent::default();
            let mut after_update = showtimes_events::m::ProjectUpdatedDataEvent::default();

            before_update.set_status(prj_info.status);
            prj_info.status = new_status.into();
            after_update.set_status(prj_info.status);

            // Save the project
            // TODO: Propagate error properly
            prj_handler.save(&mut prj_info, None).await?;

            // Save search results
            let meili = ctx.data_unchecked::<SearchClientShared>().clone();
            let proj_search = vec![showtimes_search::models::Project::from(&prj_info)];
            let o_project_index = meili.index(showtimes_search::models::Project::index_name());
            let task_search = tokio::task::spawn(async move {
                match o_project_index
                    .add_or_update(
                        &proj_search,
                        Some(showtimes_search::models::Project::primary_key()),
                    )
                    .await
                {
                    Ok(o_project_task) => {
                        match o_project_task.wait_for_completion(&meili, None, None).await {
                            Ok(_) => Ok(()),
                            Err(e) => Err(e),
                        }
                    }
                    Err(e) => Err(e),
                }
            });

            // Save the project update event
            let task_events = ctx
                .data_unchecked::<showtimes_events::SharedSHClickHouse>()
                .create_event_async(
                    showtimes_events::m::EventKind::ProjectUpdated,
                    showtimes_events::m::ProjectUpdatedEvent::new(
                        prj_info.id,
                        before_update,
                        after_update,
                    ),
                    if user.kind == UserKind::Owner {
                        None
                    } else {
                        Some(user.id.to_string())
                    },
                );

            // Wait for all tasks
            execute_search_events(task_search, task_events).await?;

            // Return the updated project
            let prj_gql = ProjectGQL::from(&prj_info);
            return Ok(prj_gql);
        }
        (_, Some(new_status)) => {
            let new_status_db: showtimes_db::m::ProjectStatus = new_status.into();
            if prj_info.status != new_status_db {
                // STOP!
                let mut before_update = showtimes_events::m::ProjectUpdatedDataEvent::default();
                let mut after_update = showtimes_events::m::ProjectUpdatedDataEvent::default();

                before_update.set_status(prj_info.status);
                prj_info.status = new_status_db;
                after_update.set_status(prj_info.status);

                // Save the project
                // TODO: Propagate error properly
                prj_handler.save(&mut prj_info, None).await?;

                // Save search results
                let meili = ctx.data_unchecked::<SearchClientShared>().clone();
                let proj_search = vec![showtimes_search::models::Project::from(&prj_info)];
                let o_project_index = meili.index(showtimes_search::models::Project::index_name());
                let task_search = tokio::task::spawn(async move {
                    match o_project_index
                        .add_or_update(
                            &proj_search,
                            Some(showtimes_search::models::Project::primary_key()),
                        )
                        .await
                    {
                        Ok(o_project_task) => {
                            match o_project_task.wait_for_completion(&meili, None, None).await {
                                Ok(_) => Ok(()),
                                Err(e) => Err(e),
                            }
                        }
                        Err(e) => Err(e),
                    }
                });

                // Save the project update event
                let task_events = ctx
                    .data_unchecked::<showtimes_events::SharedSHClickHouse>()
                    .create_event_async(
                        showtimes_events::m::EventKind::ProjectUpdated,
                        showtimes_events::m::ProjectUpdatedEvent::new(
                            prj_info.id,
                            before_update,
                            after_update,
                        ),
                        if user.kind == UserKind::Owner {
                            None
                        } else {
                            Some(user.id.to_string())
                        },
                    );

                // Wait for all tasks
                execute_search_events(task_search, task_events).await?;

                // Return the updated project
                let prj_gql = ProjectGQL::from(&prj_info);
                return Ok(prj_gql);
            }

            // Ignore since it's the same
        }
        // Ignore since the rest should be None
        _ => {}
    };

    // Preload all user if we need to update assignees
    let mut loaded_users: HashMap<showtimes_shared::ulid::Ulid, showtimes_db::m::User> =
        HashMap::new();
    if let Some(assignees_update) = &input.assignees {
        let user_ids_keys = assignees_update
            .iter()
            .filter_map(|a| a.id.as_ref().map(|id| **id))
            .collect::<Vec<showtimes_shared::ulid::Ulid>>();
        loaded_users = usr_loader.load_many(user_ids_keys).await?;
    }

    let mut metadata_sync: Option<ExternalMediaFetchResult> = None;

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
                    return GQLError::new(
                        format!(
                            "Provider `{}` not supported for metadata sync",
                            provider.kind()
                        ),
                        GQLErrorCode::MetadataUnknownSource,
                    )
                    .extend(|e| {
                        e.set("id", provider.id());
                        e.set("provider", provider.kind().to_string());
                    })
                    .into();
                }
            };

            metadata_sync = Some(metadata_res);
        }
    }

    // TODO: Validate propagate error
    let (project_event, episode_event) = update_single_project(
        ctx,
        &mut prj_info,
        &input,
        metadata_sync.as_ref(),
        &loaded_users,
        true,
    )
    .await?;

    // Save the project
    // TODO: Propagate error properly
    prj_handler.save(&mut prj_info, None).await?;

    // Search results
    let mut all_project_search: Vec<showtimes_search::models::Project> =
        vec![showtimes_search::models::Project::from(&prj_info)];

    // Project update events
    let mut all_project_update_events = vec![project_event];
    let mut all_project_episodes_events = vec![];
    all_project_episodes_events.extend(episode_event);

    // TODO: Validate propagate error
    let mut other_projects = fetch_project_collaborators(ctx, &prj_info).await?;

    for o_project in other_projects.iter_mut() {
        // TODO: Validate propagate error
        let (project_event, episode_event) = update_single_project(
            ctx,
            o_project,
            &input,
            metadata_sync.as_ref(),
            &loaded_users,
            false,
        )
        .await?;

        // Save the project
        // TODO: Propagate error properly
        prj_handler.save(o_project, None).await?;

        // Create search results
        let project_search = showtimes_search::models::Project::from(&prj_info);
        all_project_search.push(project_search);

        // Push events
        all_project_update_events.push(project_event);
        all_project_episodes_events.extend(episode_event);
    }

    // Save the search results
    let meili = ctx.data_unchecked::<SearchClientShared>().clone();
    let o_project_index = meili.index(showtimes_search::models::Project::index_name());
    let task_search = tokio::task::spawn(async move {
        match o_project_index
            .add_or_update(
                &all_project_search,
                Some(showtimes_search::models::Project::primary_key()),
            )
            .await
        {
            Ok(o_project_task) => {
                match o_project_task.wait_for_completion(&meili, None, None).await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        }
    });

    // Save the project update event
    let task_project_events = ctx
        .data_unchecked::<showtimes_events::SharedSHClickHouse>()
        .create_event_many_async(
            showtimes_events::m::EventKind::ProjectUpdated,
            all_project_update_events,
            if user.kind == UserKind::Owner {
                None
            } else {
                Some(user.id.to_string())
            },
        );
    // Save the episode update event
    let task_episode_events = ctx
        .data_unchecked::<showtimes_events::SharedSHClickHouse>()
        .create_event_many_async(
            showtimes_events::m::EventKind::ProjectEpisodes,
            all_project_episodes_events,
            if user.kind == UserKind::Owner {
                None
            } else {
                Some(user.id.to_string())
            },
        );

    // Wait for all events to finish
    // TODO: Propagate error properly
    let (t_a, t_b, t_c) = tokio::try_join!(task_search, task_project_events, task_episode_events)?;
    t_a?;
    t_b?;
    t_c?;

    // Return the updated project
    let prj_gql = ProjectGQL::from(&prj_info);
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
    let prj_handler = ProjectHandler::new(db);

    // Fetch project
    let mut prj_info = prj_loader.load_one(*id).await?.ok_or_else(|| {
        GQLError::new("Project not found", GQLErrorCode::ProjectNotFound)
            .extend(|e| e.set("id", id.to_string()))
    })?;

    // Check perms
    check_permissions(ctx, prj_info.creator, &user, Some(prj_info.id)).await?;

    if prj_info.status == showtimes_db::m::ProjectStatus::Archived {
        return GQLError::new("Project is archived", GQLErrorCode::ProjectArchived)
            .extend(|e| e.set("id", id.to_string()))
            .into();
    }

    fn update_project_inner(
        project: &mut showtimes_db::m::Project,
        count: u64,
    ) -> showtimes_events::m::ProjectUpdatedEvent {
        // Add episodes from the last episode
        let mut sorted_episodes = project.progress.clone();
        sorted_episodes.sort();
        let last_episode = sorted_episodes
            .last()
            .expect("No episodes exist on this project");
        let last_air_date = last_episode.aired;
        let new_episodes = ((last_episode.number + 1)..=(last_episode.number + count))
            .enumerate()
            .map(|(idx, n)| {
                let next_air_date =
                    last_air_date.map(|d| d + chrono::Duration::weeks((idx + 1) as i64));

                let mut ep =
                    showtimes_db::m::EpisodeProgress::new_with_roles(n, false, &project.roles);
                ep.set_aired(next_air_date);
                ep
            })
            .collect::<Vec<showtimes_db::m::EpisodeProgress>>();

        let episode_events: Vec<showtimes_events::m::ProjectUpdatedEpisodeDataEvent> = new_episodes
            .iter()
            .map(|episode| {
                let mut event =
                    showtimes_events::m::ProjectUpdatedEpisodeDataEvent::added(episode.number);
                if let Some(aired) = episode.aired {
                    event.set_aired(aired.timestamp());
                }

                event
            })
            .collect();

        // Extend, sort, then replace
        sorted_episodes.extend(new_episodes);
        sorted_episodes.sort();
        project.progress = sorted_episodes;

        let before_project = showtimes_events::m::ProjectUpdatedDataEvent::default();
        let mut after_project = showtimes_events::m::ProjectUpdatedDataEvent::default();

        after_project.set_progress(&episode_events);

        showtimes_events::m::ProjectUpdatedEvent::new(project.id, before_project, after_project)
    }

    // Events and search
    let mut all_events_content = vec![];
    let mut all_search_contents = vec![];

    if prj_info.progress.is_empty() {
        return GQLError::new(
            "Project has no episodes",
            GQLErrorCode::ProjectEmptyEpisodes,
        )
        .extend(|e| {
            e.set("id", id.to_string());
        })
        .into();
    }

    // Save the project
    let project_event = update_project_inner(&mut prj_info, count);
    // TODO: Propagate error properly
    prj_handler.save(&mut prj_info, None).await?;

    // Push to search and events
    all_search_contents.push(showtimes_search::models::Project::from(&prj_info));
    all_events_content.push(project_event);

    // Fetch collaborators
    // TODO: Validate propagate error
    let mut other_projects = fetch_project_collaborators(ctx, &prj_info).await?;
    for project in other_projects.iter_mut() {
        if project.progress.is_empty() {
            return GQLError::new(
                "Collab project has no episodes",
                GQLErrorCode::ProjectEmptyEpisodes,
            )
            .extend(|e| {
                e.set("id", project.id.to_string());
                e.set("original", id.to_string());
            })
            .into();
        }

        let project_event = update_project_inner(project, count);
        // Save other project
        // TODO: Propagate error properly
        prj_handler.save(project, None).await?;

        // Push to search and events
        all_search_contents.push(showtimes_search::models::Project::from(project));
        all_events_content.push(project_event);
    }

    let meili = ctx.data_unchecked::<SearchClientShared>().clone();
    let task_search = tokio::task::spawn(async move {
        let search_index = meili.index(showtimes_search::models::Project::index_name());
        match search_index
            .add_or_update(
                &all_search_contents,
                Some(showtimes_search::models::Project::primary_key()),
            )
            .await
        {
            Ok(search_task) => match search_task.wait_for_completion(&meili, None, None).await {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        }
    });
    let task_events = ctx
        .data_unchecked::<showtimes_events::SharedSHClickHouse>()
        .create_event_many_async(
            showtimes_events::m::EventKind::ProjectUpdated,
            all_events_content,
            if user.kind == UserKind::Owner {
                None
            } else {
                Some(user.id.to_string())
            },
        );

    // Execute the tasks
    execute_search_events(task_search, task_events).await?;

    // Return the updated project
    let prj_gql = ProjectGQL::from(&prj_info);

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
    let prj_handler = ProjectHandler::new(db);

    // Fetch project
    let mut prj_info = prj_loader.load_one(*id).await?.ok_or_else(|| {
        GQLError::new("Project not found", GQLErrorCode::ProjectNotFound)
            .extend(|e| e.set("id", id.to_string()))
    })?;

    // Check perms
    check_permissions(ctx, prj_info.creator, &user, Some(prj_info.id)).await?;

    if prj_info.status == showtimes_db::m::ProjectStatus::Archived {
        return GQLError::new("Project is archived", GQLErrorCode::ProjectArchived)
            .extend(|e| e.set("id", id.to_string()))
            .into();
    }

    fn update_project_inner(
        project: &mut showtimes_db::m::Project,
        episodes: &[ProgressCreateInputGQL],
    ) -> showtimes_events::m::ProjectUpdatedEvent {
        let mut before_project = showtimes_events::m::ProjectUpdatedDataEvent::default();
        let mut after_project = showtimes_events::m::ProjectUpdatedDataEvent::default();

        // Add episodes
        for episode in episodes {
            let exist_mut = project.find_episode_mut(episode.number);
            if let Some(repl_mut) = exist_mut {
                let mut aired_before =
                    showtimes_events::m::ProjectUpdatedEpisodeDataEvent::updated(repl_mut.number);
                let mut aired_after = aired_before.clone();
                if let Some(aired_at) = repl_mut.aired {
                    aired_before.set_aired(aired_at.timestamp());
                }
                if let Some(aired) = &episode.aired {
                    repl_mut.set_aired(Some(**aired));
                }
                if let Some(aired_at) = repl_mut.aired {
                    aired_after.set_aired(aired_at.timestamp());
                }
                before_project.add_progress(aired_before);
                after_project.add_progress(aired_after);
            } else {
                match &episode.aired {
                    Some(aired) => {
                        project.add_episode_with_number_and_airing(episode.number, **aired);
                        let mut ep_events =
                            showtimes_events::m::ProjectUpdatedEpisodeDataEvent::added(
                                episode.number,
                            );
                        ep_events.set_aired(aired.timestamp());
                        after_project.add_progress(ep_events);
                    }
                    None => {
                        project.add_episode_with_number(episode.number);
                        after_project.add_progress(
                            showtimes_events::m::ProjectUpdatedEpisodeDataEvent::added(
                                episode.number,
                            ),
                        );
                    }
                }
            }
        }

        // Sort the episodes
        project.sort_progress();

        showtimes_events::m::ProjectUpdatedEvent::new(project.id, before_project, after_project)
    }

    let project_event = update_project_inner(&mut prj_info, episodes);
    let mut all_events_content = vec![project_event];

    // Save the project
    // TODO: Propagate error properly
    prj_handler.save(&mut prj_info, None).await?;
    let mut all_search_contents = vec![showtimes_search::models::Project::from(&prj_info)];

    // Sync the collaborations
    // TODO: Validate propagate error
    let mut other_projects = fetch_project_collaborators(ctx, &prj_info).await?;

    for project in other_projects.iter_mut() {
        let project_event = update_project_inner(project, episodes);
        // Save other project
        // TODO: Propagate error properly
        prj_handler.save(project, None).await?;

        // Push to search and events
        all_search_contents.push(showtimes_search::models::Project::from(project));
        all_events_content.push(project_event);
    }

    // Save the search results
    let meili = ctx.data_unchecked::<SearchClientShared>().clone();
    let o_project_index = meili.index(showtimes_search::models::Project::index_name());
    let task_search = tokio::task::spawn(async move {
        match o_project_index
            .add_or_update(
                &all_search_contents,
                Some(showtimes_search::models::Project::primary_key()),
            )
            .await
        {
            Ok(o_project_task) => {
                match o_project_task.wait_for_completion(&meili, None, None).await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        }
    });

    // Create task events
    let task_events = ctx
        .data_unchecked::<showtimes_events::SharedSHClickHouse>()
        .create_event_many_async(
            showtimes_events::m::EventKind::ProjectUpdated,
            all_events_content,
            if user.kind == UserKind::Owner {
                None
            } else {
                Some(user.id.to_string())
            },
        );

    // Save the project update event
    execute_search_events(task_search, task_events).await?;

    // Return the updated project
    let prj_gql = ProjectGQL::from(&prj_info);
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

    // Fetch project
    let mut prj_info = prj_loader.load_one(*id).await?.ok_or_else(|| {
        GQLError::new("Project not found", GQLErrorCode::ProjectNotFound)
            .extend(|e| e.set("id", id.to_string()))
    })?;

    // Check perms
    check_permissions(ctx, prj_info.creator, &user, Some(prj_info.id)).await?;

    if prj_info.status == showtimes_db::m::ProjectStatus::Archived {
        return GQLError::new("Project is archived", GQLErrorCode::ProjectArchived)
            .extend(|e| e.set("id", id.to_string()))
            .into();
    }

    fn update_project_inner(
        project: &mut showtimes_db::m::Project,
        episodes: &[u64],
    ) -> showtimes_events::m::ProjectUpdatedEvent {
        let mut to_be_removed: Vec<u64> = vec![];
        let mut before_project = showtimes_events::m::ProjectUpdatedDataEvent::default();

        for episode in episodes {
            let find_episode = project.find_episode(*episode);
            if find_episode.is_some() {
                to_be_removed.push(*episode);
                before_project.add_progress(
                    showtimes_events::m::ProjectUpdatedEpisodeDataEvent::removed(*episode),
                );
            }
        }

        // Remove episodes marked
        project
            .progress
            .retain(|ep| !to_be_removed.contains(&ep.number));

        // Sort the episodes
        project.sort_progress();

        let after_project = showtimes_events::m::ProjectUpdatedDataEvent::default();

        showtimes_events::m::ProjectUpdatedEvent::new(project.id, before_project, after_project)
    }

    let project_event = update_project_inner(&mut prj_info, episodes);
    let mut all_events_content = vec![project_event];

    // Save the project
    let prj_handler = ProjectHandler::new(db);
    // TODO: Propagate error properly
    prj_handler.save(&mut prj_info, None).await?;

    // Create search results
    let mut all_search_contents = vec![showtimes_search::models::Project::from(&prj_info)];

    // Sync the collaborations
    // TODO: Validate propagate error
    let mut other_projects = fetch_project_collaborators(ctx, &prj_info).await?;

    for project in other_projects.iter_mut() {
        let project_event = update_project_inner(project, episodes);
        // Save other project
        // TODO: Propagate error properly
        prj_handler.save(project, None).await?;

        // Push to search and events
        all_search_contents.push(showtimes_search::models::Project::from(project));
        all_events_content.push(project_event);
    }

    // Save the search results
    let meili = ctx.data_unchecked::<SearchClientShared>().clone();
    let o_project_index = meili.index(showtimes_search::models::Project::index_name());
    let task_search = tokio::task::spawn(async move {
        match o_project_index
            .add_or_update(
                &all_search_contents,
                Some(showtimes_search::models::Project::primary_key()),
            )
            .await
        {
            Ok(o_project_task) => {
                match o_project_task.wait_for_completion(&meili, None, None).await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        }
    });

    // Create task events
    let task_events = ctx
        .data_unchecked::<showtimes_events::SharedSHClickHouse>()
        .create_event_many_async(
            showtimes_events::m::EventKind::ProjectUpdated,
            all_events_content,
            if user.kind == UserKind::Owner {
                None
            } else {
                Some(user.id.to_string())
            },
        );

    // Save the project update event
    execute_search_events(task_search, task_events).await?;

    // Return the updated project
    let prj_gql = ProjectGQL::from(&prj_info);
    Ok(prj_gql)
}
