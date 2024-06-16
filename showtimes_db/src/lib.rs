pub mod models;

use std::sync::{Arc, Mutex};

use futures::stream::TryStreamExt;
/// A shorthand for the models module
pub use models as m;
/// Re-export the mongodb crate
pub use mongodb;
use mongodb::{bson::doc, options::ClientOptions};

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

/// A quick macro to create a handler for a collection
macro_rules! create_handler {
    ($name:ident, $model_name:literal, $model:ty) => {
        #[derive(Debug, Clone)]
        #[doc = "A handler for the `"]
        #[doc = $model_name]
        #[doc = "` collection"]
        pub struct $name {
            /// The shared database connection
            pub db: DatabaseMutex,
            #[doc = "The shared connection for the `"]
            #[doc = $model_name]
            #[doc = "` collection"]
            pub col: CollectionMutex<$model>,
        }

        impl $name {
            /// Create a new instance of the handler
            pub fn new(db: DatabaseMutex) -> Self {
                let typed_col = db.lock().unwrap().collection::<$model>($model_name);
                Self {
                    db,
                    col: Arc::new(Mutex::new(typed_col)),
                }
            }

            #[doc = "Find all documents in the `"]
            #[doc = $model_name]
            #[doc = "` collection"]
            pub async fn find_all(&self) -> anyhow::Result<Vec<$model>> {
                let col = self.col.lock().unwrap();
                let mut cursor = col.find(None, None).await?;
                let mut results = Vec::new();

                while let Some(result) = cursor.try_next().await? {
                    results.push(result);
                }

                Ok(results)
            }

            #[doc = "Find a document by its id in the `"]
            #[doc = $model_name]
            #[doc = "` collection"]
            pub async fn find_by_id(&self, id: &str) -> anyhow::Result<Option<$model>> {
                let col = self.col.lock().unwrap();
                let filter = doc! { "_id": id };
                let result = col.find_one(filter, None).await?;
                Ok(result)
            }

            #[doc = "Find document by a filter in the `"]
            #[doc = $model_name]
            #[doc = "` collection"]
            pub async fn find_by(
                &self,
                filter: mongodb::bson::Document,
            ) -> anyhow::Result<Option<$model>> {
                let col = self.col.lock().unwrap();
                let result = col.find_one(filter, None).await?;
                Ok(result)
            }

            #[doc = "Find all documents by a filter in the `"]
            #[doc = $model_name]
            #[doc = "` collection"]
            pub async fn find_all_by(
                &self,
                filter: mongodb::bson::Document,
            ) -> anyhow::Result<Vec<$model>> {
                let col = self.col.lock().unwrap();
                let mut cursor = col.find(filter, None).await?;
                let mut results = Vec::new();

                while let Some(result) = cursor.try_next().await? {
                    results.push(result);
                }

                Ok(results)
            }

            #[doc = "Insert a document in the `"]
            #[doc = $model_name]
            #[doc = "` collection"]
            pub async fn insert(&self, docs: Vec<$model>) -> anyhow::Result<()> {
                let col = self.col.lock().unwrap();
                col.insert_many(docs, None).await?;
                Ok(())
            }

            // TODO: Add `update` or `upsert` method, `delete` method, etc.
            // TODO: A more complex query method can be done manually by using the `col` field
        }
    };
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

create_handler!(ProjectHandler, "ShowtimesProjects", m::Project);
create_handler!(UserHandler, "ShowtimesUsers", m::User);
create_handler!(Server, "ShowtimesServers", m::Server);
create_handler!(
    ServerCollaborationHandler,
    "ShowtimesCollaborations",
    m::ServerCollaborationSync
);
create_handler!(
    ServerCollaborationInviteHandler,
    "ShowtimesCollaborationInvites",
    m::ServerCollaborationInvite
);
