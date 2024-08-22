use std::sync::Arc;

use tokio::sync::Mutex;

pub type StorageShared = Arc<showtimes_fs::FsPool>;

#[allow(dead_code)]
#[derive(Clone)]
pub struct ShowtimesState {
    /// The `showtimes_db` database
    pub db: showtimes_db::DatabaseShared,
    /// Storage handler
    pub storage: StorageShared,
    /// Meilisearch handler
    pub meili: showtimes_search::ClientMutex,
    /// Configuration
    pub config: Arc<showtimes_shared::Config>,
    /// The GraphQL request schema
    pub schema: showtimes_gql::ShowtimesGQLSchema,
    /// The redis session handler
    pub session: showtimes_session::manager::SharedSessionManager,
    /// External metadata providers (Anilist)
    pub anilist_provider: Arc<Mutex<showtimes_metadata::AnilistProvider>>,
    /// External metadata providers (TMDb)
    pub tmdb_provider: Option<Arc<showtimes_metadata::TMDbProvider>>,
    /// External metadata providers (VNDB)
    pub vndb_provider: Option<Arc<showtimes_metadata::VndbProvider>>,
}
