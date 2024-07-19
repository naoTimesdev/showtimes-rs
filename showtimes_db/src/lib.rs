pub mod models;

use bson::doc;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::ShowModelHandler;
use futures::stream::TryStreamExt;
/// A shorthand for the models module
pub use models as m;
/// Re-export the mongodb crate
pub use mongodb;
use mongodb::options::ClientOptions;
use showtimes_derive::create_handler;

/// A type alias for a shared [`mongodb::Client`] wrapped in an [`Arc`] and [`Mutex`]
pub type ClientMutex = Arc<Mutex<mongodb::Client>>;
/// A type alias for a shared [`mongodb::Database`] wrapped in an [`Arc`] and [`Mutex`]
pub type DatabaseMutex = Arc<Mutex<mongodb::Database>>;
/// A type alias for a shared [`mongodb::Collection`] wrapped in an [`Arc`] and [`Mutex`]
pub type CollectionMutex<T> = Arc<Mutex<mongodb::Collection<T>>>;

/// Shared connection handler
pub struct Connection {
    /// The mongodb client
    pub client: ClientMutex,
    /// The `showtimes_db` database
    pub db: DatabaseMutex,
}

/// Create a connection to the MongoDB server
///
/// # Arguments
/// - `url` - The URL of the MongoDB server.
///           This is formatted as `mongodb://<host>:<port>`
pub async fn create_connection(url: &str) -> anyhow::Result<Connection> {
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

    // It works! Return the client and db with Arc<Mutex<T>>
    Ok(Connection {
        client: Arc::new(Mutex::new(client)),
        db: Arc::new(Mutex::new(db)),
    })
}

create_handler!(m::Project, ProjectHandler);
create_handler!(m::User, UserHandler);
create_handler!(m::Server, ServerHandler);
create_handler!(m::ServerCollaborationSync, CollaborationSyncHandler);
create_handler!(m::ServerCollaborationInvite, CollaborationInviteHandler);
