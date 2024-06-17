pub mod models;

use std::sync::Arc;
use tokio::sync::Mutex;

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

pub async fn create_connection(url: &str) -> anyhow::Result<Connection> {
    let mut options = ClientOptions::parse(url).await?;
    let client_name = format!("showtimes-rs-db/{}", env!("CARGO_PKG_VERSION"));

    options.app_name = Some(client_name);

    let client = mongodb::Client::with_options(options)?;

    // get showtimes_db database
    let db = client.database("showtimes_db");
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
