#![doc = include_str!("../README.md")]

use std::sync::Arc;

use meilisearch_sdk::client::Client;
pub use meilisearch_sdk::errors::Error as MeiliError;

pub mod models;

/// The shared MeiliSearch client
pub type SearchClientShared = Arc<Client>;

/// Create a connection to the MeiliSearch server
///
/// # Arguments
/// - `url` - The URL of the MeiliSearch server
/// - `api_key` - The API key of the MeiliSearch server
pub async fn create_connection(
    url: &str,
    api_key: &str,
) -> Result<SearchClientShared, meilisearch_sdk::errors::Error> {
    let client = Client::new(url, Some(api_key))?;

    // Test the connection
    client.get_version().await?;

    // It works! Return the client with Arc<Mutex<T>>
    Ok(Arc::new(client))
}
