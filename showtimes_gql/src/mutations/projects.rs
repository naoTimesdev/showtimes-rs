use std::{collections::VecDeque, sync::Arc};

use async_graphql::{dataloader::DataLoader, Error, ErrorExtensions, InputObject, Upload};
use chrono::TimeZone;
use showtimes_db::{DatabaseShared, ProjectHandler};
use showtimes_fs::FsPool;
use showtimes_metadata::m::AnilistMediaFormat;
use showtimes_search::SearchClientShared;
use tokio::{io::AsyncSeekExt, sync::Mutex};

use crate::{
    data_loader::{ServerDataLoader, UserDataLoader},
    models::{
        prelude::{DateTimeGQL, UlidGQL},
        projects::ProjectGQL,
        search::ExternalSearchSource,
    },
};

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

/// The input for roles information
#[derive(InputObject)]
pub struct ProjectRoleInputGQL {
    /// The role key
    key: String,
    /// The role long name
    name: String,
}

/// The input assignees for a project
#[derive(InputObject)]
pub struct ProjectAssigneeInputGQL {
    /// The user ID
    id: UlidGQL,
    /// The role key
    role: String,
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
        showtimes_metadata::m::AnilistMediaType::Manga => {
            let chapter_count = anilist_info
                .chapters
                .unwrap_or_else(|| input.episode.unwrap_or(0));

            chapter_count
        }
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

            current_time = current_time + chrono::Duration::weeks(1);
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
                        aired_at: first_ep.aired_at.clone(),
                    });
                }
            }

            let est_ep_u32: u32 = est_episode.try_into().unwrap();
            if last_ep.number < est_ep_u32 {
                // Extrapolate forward, use the last episode start as the basis
                let mut last_ep_start = last_ep.aired_at.clone();
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
                        number: i as u32,
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
    all_titles.sort();
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

pub async fn mutate_projects_create(
    ctx: &async_graphql::Context<'_>,
    user: showtimes_db::m::User,
    id: UlidGQL,
    input: ProjectCreateInputGQL,
) -> async_graphql::Result<ProjectGQL> {
    let srv_loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();
    let usr_loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();
    let db = ctx.data_unchecked::<DatabaseShared>();
    let storages = ctx.data_unchecked::<Arc<FsPool>>();
    let meili = ctx.data_unchecked::<SearchClientShared>();

    let srv = srv_loader.load_one(*id).await?;

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
            // Allow anyone to create a project
        }
    }

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
        let assignee = input.assignees.iter().find(|&a| &a.role == role.key());

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
            all_roles.clone(),
        );
        progress.set_aired(episode.aired_at);
        all_progress.push(progress);
    }

    let mut project = showtimes_db::m::Project::new(metadata.title, metadata.kind, srv.id)?;
    project.roles = all_roles;
    project.assignees = assignees;
    project.progress = all_progress;
    project.aliases = metadata.aliases;
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
                    &mut file_target,
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

            let mut stream = std::io::Cursor::new(cover_bytes);

            storages
                .file_stream_upload(
                    project.id,
                    &cover_key,
                    &mut stream,
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

    // Save the project
    let prj_handler = ProjectHandler::new(db);
    prj_handler.save(&mut project, None).await?;

    // Update index
    let prj_search = showtimes_search::models::Project::from(project.clone());
    prj_search.update_document(meili).await?;

    let prj_gql: ProjectGQL = project.into();

    Ok(prj_gql)
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
