use bson::doc;
use showtimes_db::{m::ShowModelHandler, MigrationHandler};

use crate::migrations::Migration;

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
        println!("[UP] Running migration: {}", migration.name());
        migration.up().await.unwrap();
        println!("[UP] Migration {} executed", migration.name());
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
            println!(
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
        println!("[UP] Updating migration {} to be current", db_update.name);
        handler.save(&mut db_update, None).await.unwrap();
    } else {
        println!("[UP] Migration {} already executed", migration.name());
    }
}

pub async fn run_migration_down(handler: &MigrationHandler, migration: Box<dyn Migration>) {
    let db_migrate = check_if_migration_exists(handler.clone(), &migration).await;

    if let Some(db_migrate) = db_migrate {
        if !db_migrate.is_current {
            println!(
                "[DOWN] Migration {} is not current, cannot be reverted",
                migration.name()
            );
        } else {
            println!("[DOWN] Running migration: {}", migration.name());
            migration.down().await.unwrap();
            handler.delete(&db_migrate).await.unwrap();
            println!(
                "[DOWN] Migration {} reverted, updating other models",
                migration.name()
            );
            let mut all_migrations = handler.find_all().await.unwrap();
            all_migrations.sort_by(|a, b| a.ts.cmp(&b.ts));
            if let Some(last_migration) = all_migrations.last() {
                let mut last_migration = last_migration.clone();
                last_migration.is_current = true;
                println!(
                    "[DOWN] Updating migration {} to be current",
                    last_migration.name
                );
                handler.save(&mut last_migration, None).await.unwrap();
            }
        }
    } else {
        println!("[DOWN] Migration {} not found", migration.name());
    }
}
