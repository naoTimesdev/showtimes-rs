//! The provider for TMDb source.
//!
//! This is incomplete and only made to support what Showtimes needed.

use crate::{
    errors::MetadataResult,
    models::{TMDbErrorResponse, TMDbMovieResult, TMDbMultiResponse, TMDbMultiResult},
};

const TMDB_API_URL: &str = "https://api.themoviedb.org/3";

/// The main client that provide data from TMDb
#[derive(Debug, Clone)]
pub struct TMDbProvider {
    client: reqwest::Client,
}

impl TMDbProvider {
    /// Create a new TMDb provider
    ///
    /// # Arguments
    /// * `access_token` - The TMDb API access token
    pub fn new(access_token: impl Into<String>) -> Self {
        let ua_bind = reqwest::header::HeaderValue::from_str(&format!(
            "showtimes-rs-metadata/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .unwrap();
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(reqwest::header::USER_AGENT, ua_bind);
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", access_token.into()))
                .expect("Failed to build the Auth header for TMDb API"),
        );

        let client = reqwest::ClientBuilder::new()
            .http2_adaptive_window(true)
            .default_headers(headers)
            .use_rustls_tls()
            .build()
            .expect("Failed to build reqwest client for TMDb API");

        TMDbProvider { client }
    }

    async fn request<T>(
        &self,
        path: &str,
        query_params: &[(&str, &str)],
    ) -> Result<T, TMDbErrorResponse>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut req = self.client.get(format!("{}/{}", TMDB_API_URL, path));

        for (key, value) in query_params {
            req = req.query(&[(*key, *value)]);
        }

        let send_req = req.send().await;

        match send_req {
            Ok(send_req) => {
                if send_req.status().is_success() {
                    let success = send_req.json::<T>().await.map_err(|e| TMDbErrorResponse {
                        status_code: -110,
                        status_message: e.to_string(),
                    })?;

                    Ok(success)
                } else {
                    // parse the error
                    let error = send_req.json::<TMDbErrorResponse>().await.map_err(|e| {
                        TMDbErrorResponse {
                            status_code: -120,
                            status_message: e.to_string(),
                        }
                    })?;

                    Err(error)
                }
            }
            Err(e) => Err(TMDbErrorResponse {
                status_code: -100,
                status_message: e.to_string(),
            }),
        }
    }

    /// Search for a tv, movie, anything
    ///
    /// This will also search for people and companies, so make sure to filter the results.
    ///
    /// # Arguments
    /// * `query` - The query to search for
    pub async fn search(&self, query: &str) -> MetadataResult<Vec<TMDbMultiResult>> {
        let response: TMDbMultiResponse<TMDbMultiResult> = self
            .request(
                "search/multi",
                &[("query", query), ("include_adult", "true")],
            )
            .await?;

        Ok(response.results)
    }

    /// Search for a movie
    ///
    /// # Arguments
    /// * `query` - The query to search for
    pub async fn search_movie(&self, query: &str) -> MetadataResult<Vec<TMDbMovieResult>> {
        let response: TMDbMultiResponse<TMDbMovieResult> = self
            .request(
                "search/movie",
                &[("query", query), ("include_adult", "true")],
            )
            .await?;

        Ok(response.results)
    }

    /// Get specific movie details
    ///
    /// # Arguments
    /// * `id` - The movie ID
    pub async fn get_movie_details(&self, id: i32) -> MetadataResult<TMDbMovieResult> {
        let response: TMDbMovieResult = self.request(&format!("movie/{}", id), &[]).await?;

        Ok(response)
    }
}
