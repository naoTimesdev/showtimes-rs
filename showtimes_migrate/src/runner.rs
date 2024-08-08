use bson::doc;
use showtimes_db::{m::ShowModelHandler, MigrationHandler};

use crate::{common::env_or_exit, migrations::Migration};

async fn check_if_migration_exists(
    handler: MigrationHandler,
    migration: &Box<dyn Migration>,
) -> Option<showtimes_db::m::Migration> {
    let name = migration.name();

    handler.find_by(doc! { "name": name }).await.unwrap()
}

pub async fn run_migration_list(
    handler: &MigrationHandler,
    migrations: Vec<Box<dyn Migration>>,
    detailed: bool,
) {
    let db_migrations = if detailed {
        handler.find_all().await.unwrap()
    } else {
        vec![]
    };

    println!("Migrations:");
    for migration in migrations.iter() {
        let db_migration = db_migrations.iter().find(|m| m.name == migration.name());

        if let Some(db_migration) = db_migration {
            println!(
                "[X] {} - {} ({})",
                migration.name(),
                db_migration.ts,
                if db_migration.is_current { "X" } else { "-" },
            );
        } else {
            println!("[ ] {} - {} (?)", migration.name(), migration.timestamp());
        }
    }
}

pub async fn run_migration_up(handler: &MigrationHandler, migration: Box<dyn Migration>) {
    let db_migrate = check_if_migration_exists(handler.clone(), &migration).await;

    if db_migrate.is_none() {
        tracing::info!("[UP] Running migration: {}", migration.name());
        migration.up().await.unwrap();
        tracing::info!("[UP] Migration {} executed", migration.name());
        // Save the migration to the database
        let mut db_update =
            showtimes_db::m::Migration::new(migration.name(), migration.timestamp());
        // Change old migration to not current
        let mut old_migrations = handler.find_all().await.unwrap();
        let mut current_id = None;
        for old_migration in old_migrations.iter_mut() {
            old_migration.is_current = false;
            tracing::info!(
                "[UP] Updating migration {} to not current",
                old_migration.name
            );
            handler.save(old_migration, None).await.unwrap();
            if old_migration.name == migration.name() {
                current_id = old_migration.id().clone();
            }
        }

        if let Some(id) = current_id {
            db_update.set_id(id);
        }

        // Save current
        tracing::info!("[UP] Updating migration {} to be current", db_update.name);
        handler.save(&mut db_update, None).await.unwrap();
    } else {
        tracing::warn!("[UP] Migration {} already executed", migration.name());
    }
}

pub async fn run_migration_down(handler: &MigrationHandler, migration: Box<dyn Migration>) {
    let db_migrate = check_if_migration_exists(handler.clone(), &migration).await;

    if let Some(db_migrate) = db_migrate {
        if !db_migrate.is_current {
            tracing::info!(
                "[DOWN] Migration {} is not current, cannot be reverted",
                migration.name()
            );
        } else {
            tracing::info!("[DOWN] Running migration: {}", migration.name());
            migration.down().await.unwrap();
            handler.delete(&db_migrate).await.unwrap();
            tracing::info!(
                "[DOWN] Migration {} reverted, updating other models",
                migration.name()
            );
            let mut all_migrations = handler.find_all().await.unwrap();
            all_migrations.sort_by(|a, b| a.ts.cmp(&b.ts));
            if let Some(last_migration) = all_migrations.last() {
                let mut last_migration = last_migration.clone();
                last_migration.is_current = true;
                tracing::info!(
                    "[DOWN] Updating migration {} to be current",
                    last_migration.name
                );
                handler.save(&mut last_migration, None).await.unwrap();
            }
        }
    } else {
        tracing::warn!("[DOWN] Migration {} not found", migration.name());
    }
}

async fn meili_create_index(client: &showtimes_search::ClientMutex) -> anyhow::Result<()> {
    tracing::info!("Creating or getting Meilisearch indexes...");
    // This will create the index if it doesn't exist
    showtimes_search::models::Project::get_index(client).await?;
    showtimes_search::models::Server::get_index(client).await?;
    showtimes_search::models::User::get_index(client).await?;
    showtimes_search::models::ServerCollabSync::get_index(client).await?;
    showtimes_search::models::ServerCollabInvite::get_index(client).await?;

    Ok(())
}

async fn meili_fixup_index(client: &showtimes_search::ClientMutex) -> anyhow::Result<()> {
    tracing::info!("Fixing Meilisearch indexes schemas...");
    showtimes_search::models::Project::update_schema(client).await?;
    showtimes_search::models::Server::update_schema(client).await?;
    showtimes_search::models::User::update_schema(client).await?;
    showtimes_search::models::ServerCollabSync::update_schema(client).await?;
    showtimes_search::models::ServerCollabInvite::update_schema(client).await?;

    Ok(())
}

pub async fn run_meilisearch_fix() -> anyhow::Result<()> {
    let meili_url = env_or_exit("MEILI_URL");
    let meili_key = env_or_exit("MEILI_KEY");

    tracing::info!("Creating Meilisearch client instances...");
    let client = showtimes_search::create_connection(&meili_url, &meili_key).await?;

    meili_create_index(&client).await?;
    meili_fixup_index(&client).await?;

    tracing::info!("Meilisearch indexes fixed");
    Ok(())
}

pub async fn run_meilisearch_reindex(conn: &showtimes_db::Connection) -> anyhow::Result<()> {
    let meili_url = env_or_exit("MEILI_URL");
    let meili_key = env_or_exit("MEILI_KEY");

    tracing::info!("Creating Meilisearch client instances...");
    let client = showtimes_search::create_connection(&meili_url, &meili_key).await?;

    meili_create_index(&client).await?;

    let client_lock = client.lock().await;

    // Reindex all models
    tracing::info!("Reindexing all models...");

    tracing::info!("Reindexing users...");
    let user_db = showtimes_db::UserHandler::new(conn.db.clone()).await;
    let users = user_db.find_all().await.unwrap();

    let mapped_users: Vec<showtimes_search::models::User> = users
        .iter()
        .map(|usr| usr.clone().into())
        .collect::<Vec<_>>();

    tracing::info!(" Committing users to Meilisearch...");
    let m_user_commit = client_lock
        .index(showtimes_search::models::User::index_name())
        .add_or_replace(
            &mapped_users,
            Some(showtimes_search::models::User::primary_key()),
        )
        .await?;
    tracing::info!(
        "  Waiting for user commit task to complete: {}",
        &m_user_commit.task_uid
    );
    m_user_commit
        .wait_for_completion(&*client_lock, None, None)
        .await?;

    tracing::info!("Reindexing servers...");
    let server_db = showtimes_db::ServerHandler::new(conn.db.clone()).await;
    let servers = server_db.find_all().await.unwrap();

    let mapped_servers: Vec<showtimes_search::models::Server> = servers
        .iter()
        .map(|srv| srv.clone().into())
        .collect::<Vec<_>>();

    tracing::info!(" Committing servers to Meilisearch...");
    let m_server_commit = client_lock
        .index(showtimes_search::models::Server::index_name())
        .add_or_replace(
            &mapped_servers,
            Some(showtimes_search::models::Server::primary_key()),
        )
        .await?;
    tracing::info!(
        "  Waiting for server commit task to complete: {}",
        &m_server_commit.task_uid
    );
    m_server_commit
        .wait_for_completion(&*client_lock, None, None)
        .await?;

    tracing::info!("Reindexing projects...");
    let project_db = showtimes_db::ProjectHandler::new(conn.db.clone()).await;
    let projects = project_db.find_all().await.unwrap();

    let mapped_projects: Vec<showtimes_search::models::Project> = projects
        .iter()
        .map(|prj| prj.clone().into())
        .collect::<Vec<_>>();

    tracing::info!(" Committing projects to Meilisearch...");
    let m_project_commit = client_lock
        .index(showtimes_search::models::Project::index_name())
        .add_or_replace(
            &mapped_projects,
            Some(showtimes_search::models::Project::primary_key()),
        )
        .await?;
    tracing::info!(
        "  Waiting for project commit task to complete: {}",
        &m_project_commit.task_uid
    );
    m_project_commit
        .wait_for_completion(&*client_lock, None, None)
        .await?;

    tracing::info!("Reindexing server collab syncs...");
    let server_collab_sync_db = showtimes_db::CollaborationSyncHandler::new(conn.db.clone()).await;
    let server_collab_syncs = server_collab_sync_db.find_all().await.unwrap();

    let mapped_server_collab_syncs: Vec<showtimes_search::models::ServerCollabSync> =
        server_collab_syncs
            .iter()
            .map(|scs| scs.clone().into())
            .collect::<Vec<_>>();

    tracing::info!(" Committing server collab syncs to Meilisearch...");
    let m_server_collab_sync_commit = client_lock
        .index(showtimes_search::models::ServerCollabSync::index_name())
        .add_or_replace(
            &mapped_server_collab_syncs,
            Some(showtimes_search::models::ServerCollabSync::primary_key()),
        )
        .await?;
    tracing::info!(
        "  Waiting for server collab sync commit task to complete: {}",
        &m_server_collab_sync_commit.task_uid
    );
    m_server_collab_sync_commit
        .wait_for_completion(&*client_lock, None, None)
        .await?;

    tracing::info!("Reindexing server collab invites...");
    let server_collab_invite_db =
        showtimes_db::CollaborationInviteHandler::new(conn.db.clone()).await;
    let server_collab_invites = server_collab_invite_db.find_all().await.unwrap();

    let mapped_server_collab_invites: Vec<showtimes_search::models::ServerCollabInvite> =
        server_collab_invites
            .iter()
            .map(|sci| sci.clone().into())
            .collect::<Vec<_>>();

    tracing::info!(" Committing server collab invites to Meilisearch...");
    let m_server_collab_invite_commit = client_lock
        .index(showtimes_search::models::ServerCollabInvite::index_name())
        .add_or_replace(
            &mapped_server_collab_invites,
            Some(showtimes_search::models::ServerCollabInvite::primary_key()),
        )
        .await?;
    tracing::info!(
        "  Waiting for server collab invite commit task to complete: {}",
        &m_server_collab_invite_commit.task_uid
    );
    m_server_collab_invite_commit
        .wait_for_completion(&*client_lock, None, None)
        .await?;

    tracing::info!("All models reindexed");

    Ok(())
}
