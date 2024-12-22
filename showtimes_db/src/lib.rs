#![doc = include_str!("../README.md")]

pub mod errors;
pub mod models;

use bson::doc;
use std::sync::Arc;

use crate::models::ShowModelHandler;
use futures_util::stream::TryStreamExt;
/// A shorthand for the models module
pub use models as m;
/// Re-export the mongodb crate
pub use mongodb;
use mongodb::options::ClientOptions;
use showtimes_derive::create_handler;

/// A type alias for a shared [`mongodb::Client`] wrapped in an [`Arc`]
pub type ClientShared = Arc<mongodb::Client>;
/// A type alias for a shared [`mongodb::Database`] wrapped in an [`Arc`]
pub type DatabaseShared = Arc<mongodb::Database>;
/// A type alias for a shared [`mongodb::Collection`] wrapped in an [`Arc`]
pub type CollectionShared<T> = Arc<mongodb::Collection<T>>;

/// Shared connection handler
pub struct Connection {
    /// The mongodb client
    pub client: ClientShared,
    /// The `showtimes_db` database
    pub db: DatabaseShared,
}

/// Create a connection to the MongoDB server
///
/// # Arguments
/// - `url` - The URL of the MongoDB server.
///           This is formatted as `mongodb://<host>:<port>`
pub async fn create_connection(url: &str) -> Result<Connection, mongodb::error::Error> {
    // Parse the connection string
    let mut options = ClientOptions::parse(url).await?;

    // Attach our client name
    let client_name = format!("showtimes-rs-db/{}", env!("CARGO_PKG_VERSION"));
    options.app_name = Some(client_name);

    // Create the client
    let client = mongodb::Client::with_options(options)?;

    // Get the `showtimes_db` database
    let db = client.database("showtimes_db");

    // Test the connection
    db.run_command(doc! { "ping": 1 }).await?;

    // It works! Return the client and db with Arc<T>
    Ok(Connection {
        client: Arc::new(client),
        db: Arc::new(db),
    })
}

create_handler!(m::Project, ProjectHandler);
create_handler!(m::User, UserHandler);
create_handler!(m::Server, ServerHandler);
create_handler!(m::ServerPremium, ServerPremiumHandler);
create_handler!(m::ServerCollaborationSync, CollaborationSyncHandler);
create_handler!(m::ServerCollaborationInvite, CollaborationInviteHandler);
create_handler!(m::RSSFeed, RSSFeedHandler);
create_handler!(m::Migration, MigrationHandler);
