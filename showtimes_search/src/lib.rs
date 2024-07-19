use std::sync::Arc;
use tokio::sync::Mutex;

use meilisearch_sdk::client::Client;

pub mod models;

pub type ClientMutex = Arc<Mutex<Client>>;
