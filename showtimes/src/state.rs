use std::sync::Arc;

use tokio::sync::Mutex;

pub type StorageShared = Arc<showtimes_fs::FsPool>;
pub type SharedShowtimesState = Arc<ShowtimesState>;

pub struct ShowtimesState {
    /// The `showtimes_db` database
    pub db: showtimes_db::DatabaseShared,
    /// Storage handler
    pub storage: StorageShared,
    /// Meilisearch handler
    pub meili: showtimes_search::SearchClientShared,
    /// Configuration
    pub config: Arc<showtimes_shared::Config>,
    /// The GraphQL request schema
    pub schema: crate::routes::graphql::ShowtimesGQLSchema,
    /// The redis session handler
    pub session: showtimes_session::manager::SharedSessionManager,
    /// External metadata providers (Anilist)
    pub anilist_provider: Arc<Mutex<showtimes_metadata::AnilistProvider>>,
    /// External metadata providers (TMDb)
    pub tmdb_provider: Option<Arc<showtimes_metadata::TMDbProvider>>,
    /// External metadata providers (VNDB)
    pub vndb_provider: Option<Arc<showtimes_metadata::VndbProvider>>,
    /// ClickHouse events broker
    pub clickhouse: showtimes_events::SharedSHClickHouse,
}
