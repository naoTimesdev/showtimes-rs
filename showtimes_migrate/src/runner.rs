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
        let mut old_migrations = handler
            .find_all_by(doc! { "is_current": false })
            .await
            .unwrap();
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

pub async fn run_meili_fix() -> anyhow::Result<()> {
    let meili_url = env_or_exit("MEILI_URL");
    let meili_key = env_or_exit("MEILI_KEY");

    tracing::info!("Creating Meilisearch client instances...");
    let client = showtimes_search::create_connection(&meili_url, &meili_key).await?;

    tracing::info!("Creating or getting Meilisearch indexes...");
    // This will create the index if it doesn't exist
    showtimes_search::models::Project::get_index(&client).await?;
    showtimes_search::models::Server::get_index(&client).await?;
    showtimes_search::models::User::get_index(&client).await?;
    showtimes_search::models::ServerCollabSync::get_index(&client).await?;
    showtimes_search::models::ServerCollabInvite::get_index(&client).await?;

    tracing::info!("Fixing Meilisearch indexes schemas...");
    showtimes_search::models::Project::update_schema(&client).await?;
    showtimes_search::models::Server::update_schema(&client).await?;
    showtimes_search::models::User::update_schema(&client).await?;
    showtimes_search::models::ServerCollabSync::update_schema(&client).await?;
    showtimes_search::models::ServerCollabInvite::update_schema(&client).await?;

    tracing::info!("Meilisearch indexes fixed");
    Ok(())
}
