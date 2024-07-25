use clap::Parser;
use cli::MigrationCommands;
use migrations::get_migrations;
use showtimes_db::create_connection;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
mod common;
mod migrations;
mod models;
mod runner;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "showtimes-migrate=debug,showtimes_migrate=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv()?;
    let mongodb_uri = match std::env::var("MONGODB_URI") {
        Ok(uri) => uri,
        Err(_) => {
            tracing::error!("MONGODB_URI environment variable not set");
            std::process::exit(1);
        }
    };

    let cli = cli::MigrationCli::parse();

    let connection = create_connection(&mongodb_uri).await.unwrap();
    let mut migrations = get_migrations(&connection.client);
    migrations.sort_by(|a, b| a.timestamp().cmp(&b.timestamp()));

    let migration_db = showtimes_db::MigrationHandler::new(connection.db).await;

    match cli.command {
        MigrationCommands::List { detailed } => {
            runner::run_migration_list(&migration_db, migrations, detailed).await;
        }
        MigrationCommands::Up { all, name } => {
            if all {
                for migration in migrations {
                    runner::run_migration_up(&migration_db, migration).await;
                }
            } else {
                let all_migrations = migration_db.find_all().await.unwrap();
                match name {
                    Some(name) => {
                        let migration = migrations
                            .iter()
                            .find(|&m| m.name().eq_ignore_ascii_case(&name));
                        if let Some(migration) = migration {
                            runner::run_migration_up(&migration_db, migration.clone_box()).await;
                        } else {
                            tracing::warn!("Migration {} not found", name);
                        }
                    }
                    None => {
                        let unmigrated = migrations
                            .iter()
                            .filter(|m| !all_migrations.iter().any(|db_m| db_m.name == m.name()))
                            .collect::<Vec<_>>();

                        let first_migration = unmigrated.first();
                        if let Some(&first) = first_migration {
                            runner::run_migration_up(&migration_db, first.clone_box()).await;
                        } else {
                            tracing::warn!("No migrations to run");
                        }
                    }
                }
            }
        }
        MigrationCommands::Down { all, name } => {
            if all {
                for migration in migrations.iter().rev() {
                    runner::run_migration_down(&migration_db, migration.clone_box()).await;
                }
            } else {
                match name {
                    Some(name) => {
                        let migration = migrations
                            .iter()
                            .find(|&m| m.name().eq_ignore_ascii_case(&name));
                        if let Some(migration) = migration {
                            runner::run_migration_down(&migration_db, migration.clone_box()).await;
                        } else {
                            tracing::warn!("Migration {} not found", name);
                        }
                    }
                    None => {
                        let migrated = migration_db.find_all().await.unwrap();
                        let last_migration = migrations
                            .iter()
                            .rev()
                            .find(|m| migrated.iter().any(|db_m| db_m.name == m.name()));
                        if let Some(last) = last_migration {
                            runner::run_migration_down(&migration_db, last.clone_box()).await;
                        } else {
                            tracing::warn!("No migrations to rollback");
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
