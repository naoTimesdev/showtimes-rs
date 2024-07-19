use std::sync::Arc;
use tokio::sync::Mutex;

use meilisearch_sdk::client::Client;

pub mod models;

pub type ClientMutex = Arc<Mutex<Client>>;

/// Create a connection to the MeiliSearch server
///
/// # Arguments
/// - `url` - The URL of the MeiliSearch server
/// - `api_key` - The API key of the MeiliSearch server
pub async fn create_connection(
    url: &str,
    api_key: &str,
) -> Result<ClientMutex, meilisearch_sdk::errors::Error> {
    let client = Client::new(url, Some(api_key))?;

    // Test the connection
    client.get_version().await?;

    // It works! Return the client with Arc<Mutex<T>>
    Ok(Arc::new(Mutex::new(client)))
}
