use std::{collections::BTreeMap, sync::Arc};

use ahash::{HashMap, HashMapExt};
use chrono::TimeZone;
use futures_util::TryStreamExt;
use mongodb::bson::doc;
use showtimes_db::{
    m::{
        DiscordUser, EpisodeProgress, ImageMetadata, IntegrationId, RoleAssignee, RoleStatus, User,
    },
    ClientShared, DatabaseShared, UserHandler,
};
use showtimes_fs::{
    local::LocalFs,
    s3::{S3Fs, S3FsCredentials, S3PathStyle},
};
use showtimes_shared::ulid::Ulid;

use crate::{
    common::env_or_exit,
    models::projects::{NumOrFloat, ProjectAssignee},
};

use super::Migration;

fn is_discord_snowflake(value: &str) -> bool {
    match value.parse::<u64>() {
        Ok(data) => data >= 4194304,
        Err(_) => false,
    }
}

fn strip_discord_discriminator(value: &str) -> String {
    // check 5 characters from the end
    if value.len() < 5 {
        return value.to_string();
    }

    let last_five = &value[value.len() - 5..];
    if last_five.starts_with('#') && last_five[1..].parse::<u16>().is_ok() {
        value[..value.len() - 5].to_string()
    } else {
        value.to_string()
    }
}

#[derive(Clone, Debug)]
struct ServerCollab {
    // Project ID (new ver)
    id: Ulid,
    // Project ID (old ver, Anilist)
    old_id: String,
    server_id: Ulid,
    // old linked project ID
    servers: Vec<String>,
}

#[derive(Clone)]
pub struct M20240725045840Init {
    client: ClientShared,
    db: DatabaseShared,
}

#[async_trait::async_trait]
impl Migration for M20240725045840Init {
    fn init(client: &ClientShared, db: &DatabaseShared) -> Self {
        Self {
            client: Arc::clone(client),
            db: Arc::clone(db),
        }
    }

    fn name(&self) -> &'static str {
        "M20240725045840Init"
    }

    fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc
            .with_ymd_and_hms(2024, 7, 25, 4, 58, 40)
            .unwrap()
    }

    #[inline(never)]
    async fn up(&self) -> anyhow::Result<()> {
        let meili_url = env_or_exit("MEILI_URL");
        let meili_key = env_or_exit("MEILI_KEY");
        let old_db_name = env_or_exit("OLD_DB_NAME");

        let meilisearch = M20240725045840Init::setup_meilisearch(&meili_url, &meili_key).await?;

        tracing::info!("Setting up filesystem...");
        let s3_bucket = std::env::var("S3_BUCKET").ok();
        let s3_region = std::env::var("S3_REGION").ok();
        let s3_endpoint_url = std::env::var("S3_ENDPOINT_URL").ok();
        let s3_access_key = std::env::var("S3_ACCESS_KEY").ok();
        let s3_secret_key = std::env::var("S3_SECRET_KEY").ok();
        let s3_path_style =
            std::env::var("S3_PATH_STYLE")
                .ok()
                .and_then(|style| match style.as_str() {
                    "virtual" => Some(S3PathStyle::VirtualHost),
                    "path" => Some(S3PathStyle::Path),
                    _ => None,
                });
        let local_storage = std::env::var("LOCAL_STORAGE").ok();

        let storages: showtimes_fs::FsPool = match (
            s3_bucket,
            s3_region,
            s3_endpoint_url,
            s3_access_key,
            s3_secret_key,
            local_storage,
        ) {
            (
                Some(bucket),
                Some(region),
                Some(endpoint_url),
                Some(access_key),
                Some(secret_key),
                _,
            ) => {
                tracing::info!(
                    " Creating S3Fs with region: {}, bucket: {}, endpoint: {:?}",
                    region,
                    bucket,
                    region,
                );

                let credentials = S3FsCredentials::new(&access_key, &secret_key);
                let bucket_info =
                    S3Fs::make_bucket(&bucket, &endpoint_url, &region, s3_path_style)?;
                showtimes_fs::FsPool::S3Fs(S3Fs::new(bucket_info, credentials)?)
            }
            (_, _, _, _, _, Some(directory)) => {
                let dir_path = std::path::PathBuf::from(directory);

                showtimes_fs::FsPool::LocalFs(LocalFs::new(dir_path))
            }
            _ => anyhow::bail!("No storage provided"),
        };

        tracing::info!("Initializing filesystem with {}...", storages.get_name());
        storages.init().await?;

        tracing::info!("Collecting servers...");
        let servers = self.collect_servers(&old_db_name).await?;
        tracing::info!("Found {} servers", servers.len());

        tracing::info!("Collecting valid users...");
        let users_map = self.collect_valid_users(&old_db_name, &servers).await?;

        // Commit users
        tracing::info!(
            "Found {} valid users, committing to database",
            users_map.len()
        );
        let mut users: Vec<showtimes_db::m::User> = users_map.values().cloned().collect();
        let user_handler = UserHandler::new(&self.db);
        user_handler.insert(&mut users).await?;
        tracing::info!("Committed all users data");

        tracing::info!("Processing {} servers...", servers.len());

        let mut mapped_servers: Vec<showtimes_db::m::Server> = vec![];
        // This is mapped like this
        // -> Old server ID -> ServerCollab
        let mut temp_mapping_projects = HashMap::<String, Vec<ServerCollab>>::new();
        // Target server -> source server
        let mut project_id_maps = HashMap::<String, HashMap<String, Ulid>>::new();
        let mut server_id_maps = HashMap::new();
        let mut temp_invite_maps =
            HashMap::<String, Vec<crate::models::servers::ServerCollabConfirm>>::new();
        let mut transformed_projects = vec![];
        for server in &servers {
            tracing::info!("Processing server {}...", &server.id);
            match M20240725045840Init::transform_server_data(server, &users_map) {
                Ok(t_server) => {
                    tracing::info!(
                        " Transform server {} projects ({} total)...",
                        &server.id,
                        server.anime.len()
                    );
                    let temp_projects = temp_mapping_projects.entry(server.id.clone()).or_default();
                    temp_invite_maps.insert(server.id.clone(), server.konfirmasi.clone());
                    server_id_maps.insert(server.id.clone(), t_server.id);
                    let server_maps = project_id_maps.entry(server.id.clone()).or_default();
                    for project in &server.anime {
                        match M20240725045840Init::transform_project_data(
                            &t_server.id,
                            project,
                            &users_map,
                        ) {
                            Ok(t_project) => {
                                let collab_data: Vec<String> = project
                                    .kolaborasi
                                    .iter()
                                    .filter_map(|id| {
                                        if id == &server.id {
                                            None
                                        } else {
                                            Some(id.clone())
                                        }
                                    })
                                    .collect();
                                if !collab_data.is_empty() {
                                    temp_projects.push(ServerCollab {
                                        id: t_project.id,
                                        old_id: project.id.clone(),
                                        server_id: t_server.id,
                                        servers: collab_data,
                                    })
                                }

                                server_maps.insert(project.id.clone(), t_project.id);
                                transformed_projects.push(t_project.clone());
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Error transforming project ({} srv {}): {}",
                                    &project.id,
                                    &server.id,
                                    e
                                );
                                anyhow::bail!(
                                    "Error transforming project ({} srv {}): {}",
                                    &project.id,
                                    &server.id,
                                    e
                                );
                            }
                        }
                    }

                    tracing::info!(
                        " Server {} has {} projects, transformed all possible one!",
                        &server.id,
                        server.anime.len()
                    );

                    mapped_servers.push(t_server);
                }
                Err(e) => {
                    tracing::error!("Error transforming server ({}): {}", &server.id, e);
                    if !server.anime.is_empty() {
                        tracing::warn!(" Server {} has an existing projects!", &server.id);
                    }
                    anyhow::bail!("Error transforming server ({}): {}", &server.id, e);
                }
            }
        }

        if !mapped_servers.is_empty() {
            tracing::info!("Committing all servers to database...");
            let server_handler = showtimes_db::ServerHandler::new(&self.db);
            server_handler.insert(&mut mapped_servers).await?;
        } else {
            tracing::warn!("No servers to commit");
        }

        if !transformed_projects.is_empty() {
            tracing::info!("Committing all projects to database...");
            let project_handler = showtimes_db::ProjectHandler::new(&self.db);
            project_handler.insert(&mut transformed_projects).await?;
        }

        tracing::info!("Processing server collaborations...");
        let mut transformed_collab_sync =
            M20240725045840Init::create_collaboration_sync_maps(&temp_mapping_projects).await?;

        if !transformed_collab_sync.is_empty() {
            tracing::info!("Committing server collaborations to database...");
            let collab_sync_handler = showtimes_db::CollaborationSyncHandler::new(&self.db);
            collab_sync_handler
                .insert(&mut transformed_collab_sync)
                .await?;
        }

        tracing::info!("Processing collab invites...");
        let mut transformed_invites = M20240725045840Init::create_collaboration_invite_maps(
            &project_id_maps,
            &temp_invite_maps,
            &server_id_maps,
        )
        .await?;

        if !transformed_invites.is_empty() {
            tracing::info!("Committing collab invites to database...");
            let collab_invite_handler = showtimes_db::CollaborationInviteHandler::new(&self.db);
            collab_invite_handler
                .insert(&mut transformed_invites)
                .await?;
        }

        tracing::info!("Comitting all users into search index...");
        let m_user_index = meilisearch.index(showtimes_search::models::User::index_name());
        let m_users_docs: Vec<showtimes_search::models::User> =
            users.iter().map(|user| user.clone().into()).collect();
        let m_use_commit = m_user_index
            .add_documents(
                &m_users_docs,
                Some(showtimes_search::models::User::primary_key()),
            )
            .await?;
        tracing::info!("Waiting for users search index to be completely committed...");
        m_use_commit
            .wait_for_completion(&*meilisearch, None, None)
            .await?;

        tracing::info!("Comitting all servers into search index...");
        let m_server_index = meilisearch.index(showtimes_search::models::Server::index_name());
        let m_server_docs: Vec<showtimes_search::models::Server> = mapped_servers
            .iter()
            .map(|server| server.clone().into())
            .collect::<Vec<_>>();
        let m_server_commit = m_server_index
            .add_documents(
                &m_server_docs,
                Some(showtimes_search::models::Server::primary_key()),
            )
            .await?;
        tracing::info!("Waiting for server search index to be completely committed...");
        m_server_commit
            .wait_for_completion(&*meilisearch, None, None)
            .await?;

        tracing::info!("Comitting all projects into search index...");
        let m_project_index = meilisearch.index(showtimes_search::models::Project::index_name());
        let m_project_docs: Vec<showtimes_search::models::Project> = transformed_projects
            .iter()
            .map(|project| project.clone().into())
            .collect::<Vec<_>>();
        let m_project_commit = m_project_index
            .add_documents(
                &m_project_docs,
                Some(showtimes_search::models::Project::primary_key()),
            )
            .await?;
        tracing::info!("Waiting for project search index to be completely committed...");
        m_project_commit
            .wait_for_completion(&*meilisearch, None, None)
            .await?;

        tracing::info!("Comitting all server collaborations into search index...");
        let m_collab_index =
            meilisearch.index(showtimes_search::models::ServerCollabSync::index_name());
        let m_collab_docs: Vec<showtimes_search::models::ServerCollabSync> =
            transformed_collab_sync
                .iter()
                .map(|collab| collab.clone().into())
                .collect::<Vec<_>>();
        let m_collab_commit = m_collab_index
            .add_documents(
                &m_collab_docs,
                Some(showtimes_search::models::ServerCollabSync::primary_key()),
            )
            .await?;
        tracing::info!(
            "Waiting for server collaboration search index to be completely committed..."
        );
        m_collab_commit
            .wait_for_completion(&*meilisearch, None, None)
            .await?;

        tracing::info!("Comitting all server collab invites into search index...");
        let m_invite_index =
            meilisearch.index(showtimes_search::models::ServerCollabInvite::index_name());
        let m_invite_docs: Vec<showtimes_search::models::ServerCollabInvite> = transformed_invites
            .iter()
            .map(|invite| invite.clone().into())
            .collect::<Vec<_>>();
        let m_invite_commit = m_invite_index
            .add_documents(
                &m_invite_docs,
                Some(showtimes_search::models::ServerCollabInvite::primary_key()),
            )
            .await?;
        tracing::info!(
            "Waiting for server collab invite search index to be completely committed..."
        );
        m_invite_commit
            .wait_for_completion(&*meilisearch, None, None)
            .await?;

        M20240725045840Init::setup_meilisearch_index(&meilisearch).await?;

        Ok(())
    }

    async fn down(&self) -> anyhow::Result<()> {
        let meili_url = env_or_exit("MEILI_URL");
        let meili_key = env_or_exit("MEILI_KEY");

        let meilisearch = M20240725045840Init::setup_meilisearch(&meili_url, &meili_key).await?;

        // Remove projects from the database
        let project_handler = showtimes_db::ProjectHandler::new(&self.db);
        tracing::info!("Dropping projects collection...");
        project_handler.delete_all().await?;

        // Remove servers from the database
        let server_handler = showtimes_db::ServerHandler::new(&self.db);
        tracing::info!("Dropping servers collection...");
        server_handler.delete_all().await?;

        // Remove users from the database
        let user_handler = UserHandler::new(&self.db);
        tracing::info!("Dropping users collection...");
        user_handler.delete_all().await?;

        // Remove server collaborations from the database
        let collab_sync_handler = showtimes_db::CollaborationSyncHandler::new(&self.db);
        tracing::info!("Dropping server collaborations collection...");
        collab_sync_handler.delete_all().await?;

        // Remove server collab invites from the database
        let collab_invite_handler = showtimes_db::CollaborationInviteHandler::new(&self.db);
        tracing::info!("Dropping server collab invites collection...");
        collab_invite_handler.delete_all().await?;

        // Remove projects from the search index
        tracing::info!("Dropping projects search index...");
        let m_project_index = meilisearch.index(showtimes_search::models::Project::index_name());
        let m_project_cm = m_project_index.delete_all_documents().await?;
        tracing::info!(" Waiting for projects search index to be completely deleted...");
        m_project_cm
            .wait_for_completion(&*meilisearch, None, None)
            .await?;
        tracing::info!("Projects search index has been deleted");

        // Remove servers from the search index
        tracing::info!("Dropping servers search index...");
        let m_server_index = meilisearch.index(showtimes_search::models::Server::index_name());
        let m_server_cm = m_server_index.delete_all_documents().await?;
        tracing::info!(" Waiting for servers search index to be completely deleted...");
        m_server_cm
            .wait_for_completion(&*meilisearch, None, None)
            .await?;
        tracing::info!("Servers search index has been deleted");

        // Remove users from the search index
        tracing::info!("Dropping users search index...");
        let m_user_index = meilisearch.index(showtimes_search::models::User::index_name());

        let m_user_cm = m_user_index.delete_all_documents().await?;
        tracing::info!(" Waiting for users search index to be completely deleted...");
        m_user_cm
            .wait_for_completion(&*meilisearch, None, None)
            .await?;
        tracing::info!("Users search index has been deleted");

        // Remove server collaborations from the search index
        tracing::info!("Dropping server collaborations search index...");
        let m_collab_index =
            meilisearch.index(showtimes_search::models::ServerCollabSync::index_name());

        let m_collab_cm = m_collab_index.delete_all_documents().await?;
        tracing::info!(
            " Waiting for server collaborations search index to be completely deleted..."
        );
        m_collab_cm
            .wait_for_completion(&*meilisearch, None, None)
            .await?;

        // Remove server collab invites from the search index
        tracing::info!("Dropping server collab invites search index...");
        let m_invite_index =
            meilisearch.index(showtimes_search::models::ServerCollabInvite::index_name());

        let m_invite_cm = m_invite_index.delete_all_documents().await?;
        tracing::info!(
            " Waiting for server collab invites search index to be completely deleted..."
        );
        m_invite_cm
            .wait_for_completion(&*meilisearch, None, None)
            .await?;

        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Migration> {
        Box::new(self.clone())
    }
}

impl M20240725045840Init {
    async fn setup_meilisearch(
        meili_url: &str,
        meili_key: &str,
    ) -> anyhow::Result<showtimes_search::SearchClientShared> {
        tracing::info!("Creating Meilisearch client instances...");
        let client = showtimes_search::create_connection(meili_url, meili_key).await?;

        tracing::info!("Creating Meilisearch indexes...");
        // This will create the index if it doesn't exist
        showtimes_search::models::Project::get_index(&client).await?;
        showtimes_search::models::Server::get_index(&client).await?;
        showtimes_search::models::User::get_index(&client).await?;
        showtimes_search::models::ServerCollabSync::get_index(&client).await?;
        showtimes_search::models::ServerCollabInvite::get_index(&client).await?;

        Ok(client)
    }

    async fn setup_meilisearch_index(
        client: &showtimes_search::SearchClientShared,
    ) -> anyhow::Result<()> {
        tracing::info!("Updating Meilisearch indexes schema information...");
        showtimes_search::models::Project::update_schema(client).await?;
        showtimes_search::models::Server::update_schema(client).await?;
        showtimes_search::models::User::update_schema(client).await?;
        showtimes_search::models::ServerCollabSync::update_schema(client).await?;
        showtimes_search::models::ServerCollabInvite::update_schema(client).await?;

        Ok(())
    }

    async fn collect_servers(
        &self,
        old_db_name: &str,
    ) -> anyhow::Result<Vec<crate::models::servers::Server>> {
        let client = Arc::clone(&self.client);
        let db = client.database(old_db_name);

        let coll = db.collection::<crate::models::servers::Server>("showtimesdatas");
        let cursor = coll.find(doc! {}).await?;
        let results: Vec<crate::models::servers::Server> = cursor.try_collect().await?;

        Ok(results)
    }

    async fn collect_valid_users(
        &self,
        old_db_name: &str,
        servers: &[crate::models::servers::Server],
    ) -> anyhow::Result<HashMap<String, User>> {
        let client = Arc::clone(&self.client);
        let db = client.database(old_db_name);

        let show_admins = db.collection::<crate::models::servers::ServerAdmin>("showtimesadmin");
        let ui_logins = db.collection::<crate::models::users::User>("showtimesuilogin");

        tracing::info!(" Processing from showtimesuilogin...");
        let mut ui_cursor = ui_logins
            .find(doc! { "discord_meta": {"$exists": true} })
            .await?;
        let mut all_users = HashMap::new();

        while let Some(user) = ui_cursor.try_next().await? {
            let discord_meta = user.discord_meta.unwrap();
            let discord_user = DiscordUser {
                id: discord_meta.id.clone(),
                username: strip_discord_discriminator(&discord_meta.name),
                access_token: discord_meta.access_token,
                refresh_token: discord_meta.refresh_token,
                expires_at: discord_meta.expires_at,
                avatar: None,
            };

            let created_user = match user.privilege.as_str() {
                "admin" | "owner" => User::new_admin(user.name.unwrap_or(user.id), discord_user),
                _ => User::new(user.name.unwrap_or(user.id), discord_user),
            };

            all_users.insert(discord_meta.id, created_user);
        }

        // Now create show_admins
        tracing::info!(" Processing from showtimesadmin...");
        let mut show_admins_cursor = show_admins.find(doc! {}).await?;
        while let Some(admin) = show_admins_cursor.try_next().await? {
            // If not exists, create a new user
            if !all_users.contains_key(&admin.id) {
                let mut discord_user = DiscordUser::stub();
                discord_user.id = admin.id.clone();
                let created_user = User::new(admin.id.clone(), discord_user);
                all_users.insert(admin.id.clone(), created_user.with_unregistered());
            }
        }

        // Loop through all servers to find the owners
        tracing::info!(" Processing from servers...");
        for server in servers {
            for owner in &server.serverowner {
                if !all_users.contains_key(owner) && is_discord_snowflake(owner) {
                    let mut discord_user = DiscordUser::stub();
                    discord_user.id = owner.clone();
                    let created_user = User::new(owner.clone(), discord_user);
                    all_users.insert(owner.clone(), created_user.with_unregistered());
                }
            }

            // Loop through projects in assignments
            for project in server.anime.iter() {
                let assignments = &project.assignments;
                let tl = &assignments.translator;
                let tlc = &assignments.translation_checker;
                let ed = &assignments.editor;
                let enc = &assignments.encoder;
                let tm = &assignments.timer;
                let ts = &assignments.typesetter;
                let qc = &assignments.quality_checker;

                let mut all_assignees = HashMap::new();
                if let Some((id, name)) = transform_assignee(tl) {
                    all_assignees.insert(id, name);
                }
                if let Some((id, name)) = transform_assignee(tlc) {
                    all_assignees.insert(id, name);
                }
                if let Some((id, name)) = transform_assignee(ed) {
                    all_assignees.insert(id, name);
                }
                if let Some((id, name)) = transform_assignee(enc) {
                    all_assignees.insert(id, name);
                }
                if let Some((id, name)) = transform_assignee(tm) {
                    all_assignees.insert(id, name);
                }
                if let Some((id, name)) = transform_assignee(ts) {
                    all_assignees.insert(id, name);
                }
                if let Some((id, name)) = transform_assignee(qc) {
                    all_assignees.insert(id, name);
                }

                for custom in &assignments.custom {
                    if let Some((id, name)) = transform_assignee(&custom.person) {
                        all_assignees.insert(id, name);
                    }
                }

                for (assignee, name) in all_assignees {
                    if !all_users.contains_key(&assignee) && is_discord_snowflake(&assignee) {
                        let mut discord_user = DiscordUser::stub();
                        discord_user.id = assignee.clone();
                        let name = if name.is_empty() {
                            assignee.clone()
                        } else {
                            name
                        };
                        discord_user.username = name.clone();
                        let created_user = User::new(name, discord_user);
                        all_users.insert(assignee, created_user.with_unregistered());
                    }
                }
            }
        }

        Ok(all_users)
    }

    fn transform_server_data(
        server: &crate::models::servers::Server,
        actors: &HashMap<String, User>,
    ) -> anyhow::Result<showtimes_db::m::Server> {
        let server_name = server
            .name
            .clone()
            .unwrap_or(format!("Server {}", &server.id));
        let mut integrations = vec![IntegrationId::new(
            server.id.clone(),
            showtimes_db::m::IntegrationType::DiscordGuild,
        )];

        let owners: Vec<showtimes_db::m::ServerUser> = server
            .serverowner
            .iter()
            .enumerate()
            .filter_map(|(idx, owner)| match actors.get(owner) {
                Some(user) => Some(showtimes_db::m::ServerUser::new(
                    user.id,
                    match idx {
                        0 => showtimes_db::m::UserPrivilege::Owner,
                        _ => showtimes_db::m::UserPrivilege::Admin,
                    },
                )),
                None => {
                    tracing::warn!("Owner {} not found in {}", owner, &server.id);
                    None
                }
            })
            .collect();

        if let Some(announce) = &server.announce_channel {
            if is_discord_snowflake(announce) {
                integrations.push(IntegrationId::new(
                    announce.clone(),
                    showtimes_db::m::IntegrationType::DiscordChannel,
                ));
            }
        }

        if let Some(fsdb) = server.fsdb_id {
            integrations.push(IntegrationId::new(
                fsdb.to_string(),
                showtimes_db::m::IntegrationType::FansubDB,
            ));
        }

        let server_info =
            showtimes_db::m::Server::new(server_name, owners).with_integrations(integrations);

        Ok(server_info)
    }

    fn transform_project_data(
        server_id: &Ulid,
        project: &crate::models::projects::Project,
        actors: &HashMap<String, User>,
    ) -> anyhow::Result<showtimes_db::m::Project> {
        let title = project.title.clone();
        // NAIVE assumption
        let project_kind = if project.status.len() > 1 {
            showtimes_db::m::ProjectType::Series
        } else {
            showtimes_db::m::ProjectType::Movies
        };
        let poster_url = project.poster_data.url.clone();

        let poster_info = if let Some(poster_col) = project.poster_data.color {
            showtimes_db::m::Poster::new_with_color(
                ImageMetadata::stub_with_name(poster_url),
                poster_col,
            )
        } else {
            showtimes_db::m::Poster::new(ImageMetadata::stub_with_name(poster_url))
        };

        let mut available_roles: BTreeMap<String, showtimes_db::m::Role> = vec![
            showtimes_db::m::Role::new("TL", "Translator")?.with_order(0),
            showtimes_db::m::Role::new("TLC", "Translation Checker")?.with_order(1),
            showtimes_db::m::Role::new("ED", "Editor")?.with_order(2),
            showtimes_db::m::Role::new("ENC", "Encoder")?.with_order(3),
            showtimes_db::m::Role::new("TM", "Timer")?.with_order(4),
            showtimes_db::m::Role::new("TS", "Typesetter")?.with_order(5),
            showtimes_db::m::Role::new("QC", "Quality Checker")?.with_order(6),
        ]
        .iter()
        .map(|role| (role.key().to_string(), role.clone()))
        .collect();

        for (idx, custom) in project.assignments.custom.iter().enumerate() {
            let role =
                showtimes_db::m::Role::new(&custom.key, &custom.name)?.with_order((idx + 7) as i32);
            available_roles.insert(role.key().to_string(), role);
        }

        let mapped_progress = project
            .status
            .iter()
            .map(|status| {
                let mut role_status: Vec<RoleStatus> = available_roles
                    .values()
                    .map(|role| RoleStatus::from(role.clone()))
                    .collect();

                for new_status in role_status.iter_mut() {
                    match new_status.key() {
                        "TL" => new_status.set_finished(status.progress.translation),
                        "TLC" => new_status.set_finished(status.progress.translation_check),
                        "ED" => new_status.set_finished(status.progress.editing),
                        "ENC" => new_status.set_finished(status.progress.encoding),
                        "TM" => new_status.set_finished(status.progress.timing),
                        "TS" => new_status.set_finished(status.progress.typesetting),
                        "QC" => new_status.set_finished(status.progress.quality_check),
                        _ => {
                            let custom = status
                                .progress
                                .custom
                                .iter()
                                .find(|&c| c.key == new_status.key());
                            if let Some(custom) = custom {
                                new_status.set_finished(custom.done);
                            }
                        }
                    };
                }

                let mut episode = EpisodeProgress::new(status.episode as u64, status.is_done)
                    .with_statuses(role_status);
                if let Some(airtime) = &status.airtime {
                    match airtime {
                        NumOrFloat::Num(num) => {
                            match episode.set_aired_from_unix((*num) as i64) {
                                Ok(_) => {}
                                Err(e) => {
                                    // FUCK!
                                    tracing::warn!("  Project {} has invalid number when parsing timestamp {}: {e}", &project.id, num);
                                }
                            }
                        }
                        NumOrFloat::Float(num) => {
                            // This is unix timestamp in seconds + some decimal which is milliseconds
                            let num = *num as i64;
                            match episode.set_aired_from_unix(num) {
                                Ok(_) => {}
                                Err(e) => {
                                    // FUCK!
                                    tracing::warn!("  Project {} has invalid number when parsing timestamp {}: {e}", &project.id, num);
                                }
                            }
                        }
                    };
                }

                if let Some(reason) = &status.delay_reason {
                    episode.set_delay_reason(reason.clone());
                }

                episode
            })
            .collect();

        let mut correct_roles: Vec<showtimes_db::m::Role> =
            available_roles.values().cloned().collect();
        correct_roles.sort();

        let mut assignees: BTreeMap<String, RoleAssignee> = correct_roles
            .iter()
            .map(|role| {
                let assignee = RoleAssignee::from(role);
                (role.key().to_string(), assignee)
            })
            .collect();

        assign_people(
            &project.assignments.translator.id,
            "TL",
            actors,
            &mut assignees,
        );
        assign_people(
            &project.assignments.translation_checker.id,
            "TLC",
            actors,
            &mut assignees,
        );
        assign_people(&project.assignments.editor.id, "ED", actors, &mut assignees);
        assign_people(
            &project.assignments.encoder.id,
            "ENC",
            actors,
            &mut assignees,
        );
        assign_people(&project.assignments.timer.id, "TM", actors, &mut assignees);
        assign_people(
            &project.assignments.typesetter.id,
            "TS",
            actors,
            &mut assignees,
        );
        assign_people(
            &project.assignments.quality_checker.id,
            "QC",
            actors,
            &mut assignees,
        );
        for custom in &project.assignments.custom {
            assign_people(&custom.person.id, &custom.key, actors, &mut assignees);
        }

        let proper_assignee: Vec<RoleAssignee> = assignees.values().cloned().collect();

        let mut new_project = showtimes_db::m::Project::new_with_poster_roles_assignees(
            title,
            project_kind,
            *server_id,
            poster_info,
            correct_roles,
            proper_assignee,
        )?;
        new_project.progress = mapped_progress;
        new_project.aliases = project.aliases.clone();
        let mut integrations = vec![IntegrationId::new(
            project.id.clone(),
            showtimes_db::m::IntegrationType::ProviderAnilist,
        )];
        if let Some(mal_id) = project.mal_id {
            integrations.push(IntegrationId::new(
                mal_id.to_string(),
                showtimes_db::m::IntegrationType::ProviderAnilistMal,
            ));
        }
        if let Some(fsdb_data) = &project.fsdb_data {
            if let Some(fsdb_id) = fsdb_data.id {
                integrations.push(IntegrationId::new(
                    fsdb_id.to_string(),
                    showtimes_db::m::IntegrationType::FansubDBProject,
                ));
            }
            if let Some(fsdb_anime) = fsdb_data.ani_id {
                integrations.push(IntegrationId::new(
                    fsdb_anime.to_string(),
                    showtimes_db::m::IntegrationType::FansubDBShows,
                ));
            }
        }

        new_project.integrations = integrations;
        let last_update = chrono::DateTime::<chrono::Utc>::try_from(project.last_update)
            .unwrap_or_else(|_| chrono::Utc::now());
        new_project.updated = last_update;
        // Created just do last_update minus 1 day
        new_project.created = last_update - chrono::Duration::days(1);

        Ok(new_project)
    }

    async fn create_collaboration_sync_maps(
        top_collab_info: &HashMap<String, Vec<ServerCollab>>,
    ) -> anyhow::Result<Vec<showtimes_db::m::ServerCollaborationSync>> {
        let mut mapped_temp = HashMap::new();
        for (server_id, collab_info) in top_collab_info {
            for collab in collab_info {
                let mut merged_servers = vec![server_id.clone()];
                merged_servers.extend_from_slice(&collab.servers);
                merged_servers.sort();

                // ProjectId-FirstServerId
                let key = format!("{}-{}", &collab.old_id, &merged_servers[0]);

                let all_project_ids: Vec<showtimes_db::m::ServerCollaborationSyncTarget> =
                    merged_servers
                        .iter()
                        .filter_map(|server_id| match top_collab_info.get(server_id) {
                            Some(top_c) => {
                                top_c.iter().find(|c| c.old_id == collab.old_id).map(|c| {
                                    showtimes_db::m::ServerCollaborationSyncTarget::new(
                                        c.server_id,
                                        c.id,
                                    )
                                })
                            }
                            None => None,
                        })
                        .collect();

                if all_project_ids.len() <= 1 {
                    tracing::warn!(
                        "Project collab {} has less than 2 servers, skipping...",
                        &collab.old_id
                    );
                    continue;
                }

                mapped_temp.insert(
                    key,
                    showtimes_db::m::ServerCollaborationSync::new(all_project_ids),
                );
            }
        }

        // get all values
        let all_values: Vec<showtimes_db::m::ServerCollaborationSync> =
            mapped_temp.values().cloned().collect();

        Ok(all_values)
    }

    async fn create_collaboration_invite_maps(
        project_maps: &HashMap<String, HashMap<String, Ulid>>,
        confirm_maps: &HashMap<String, Vec<crate::models::servers::ServerCollabConfirm>>,
        server_maps: &HashMap<String, Ulid>,
    ) -> anyhow::Result<Vec<showtimes_db::m::ServerCollaborationInvite>> {
        let mut all_invites = vec![];

        for (target_server_id, collab_info) in confirm_maps {
            let target_server = match server_maps.get(target_server_id) {
                Some(p) => *p,
                None => {
                    tracing::warn!("Target server {} not found", target_server_id);
                    continue;
                }
            };

            for collab in collab_info {
                let project_id = collab.anime_id.clone();
                let source_id = collab.server_id.clone();

                let source_server = match server_maps.get(&source_id) {
                    Some(p) => *p,
                    None => {
                        tracing::warn!("Source server {} not found", &source_id);
                        continue;
                    }
                };

                let source_project_id = match project_maps.get(&source_id) {
                    Some(p) => match p.get(&project_id) {
                        Some(p) => *p,
                        None => {
                            tracing::warn!(
                                "Project {} not found in source server {}",
                                &project_id,
                                &source_id
                            );
                            continue;
                        }
                    },
                    None => {
                        tracing::warn!("Source server {} not found", &source_id);
                        continue;
                    }
                };

                let target_project_id = match project_maps.get(target_server_id) {
                    Some(p) => p.get(&project_id).copied(),
                    None => None,
                };

                let invite_source = showtimes_db::m::ServerCollaborationInviteSource::new(
                    source_server,
                    source_project_id,
                );

                let invite_target = match target_project_id {
                    Some(p) => showtimes_db::m::ServerCollaborationInviteTarget::new_with_project(
                        target_server,
                        p,
                    ),
                    None => showtimes_db::m::ServerCollaborationInviteTarget::new(target_server),
                };

                all_invites.push(showtimes_db::m::ServerCollaborationInvite::new(
                    invite_source,
                    invite_target,
                ));
            }
        }

        Ok(all_invites)
    }
}

fn assign_people(
    id: &Option<String>,
    key: &str,
    actors: &HashMap<String, User>,
    assignees: &mut BTreeMap<String, RoleAssignee>,
) {
    if let Some(id) = id {
        if !is_discord_snowflake(id) {
            // skip
            return;
        }

        let assignee = assignees.get_mut(key).unwrap();
        let creator_id = actors.get(id);
        match creator_id {
            Some(creator_id) => {
                assignee.set_actor(Some(creator_id.id));
            }
            None => {
                tracing::warn!("Assignee {} not found", id);
            }
        }
    }
}

fn transform_assignee(assignee: &ProjectAssignee) -> Option<(String, String)> {
    match (&assignee.id, &assignee.name) {
        (Some(id), Some(name)) => Some((id.clone(), name.clone())),
        (Some(id), _) => Some((id.clone(), String::new())),
        _ => None,
    }
}
