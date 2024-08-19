use std::sync::Arc;

use tokio::sync::Mutex;

#[allow(dead_code)]
#[derive(Clone)]
pub struct ShowtimesState {
    /// The `showtimes_db` database
    pub db: showtimes_db::DatabaseMutex,
    /// Storage handler
    pub storage: Arc<Mutex<showtimes_fs::FsPool>>,
    /// Meilisearch handler
    pub meili: showtimes_search::ClientMutex,
    /// Configuration
    pub config: Arc<showtimes_shared::Config>,
}
