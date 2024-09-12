use crate::models::{TMDbErrorResponse, TMDbMovieResult, TMDbMultiResponse, TMDbMultiResult};

const TMDB_API_URL: &str = "https://api.themoviedb.org/3";

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
                .unwrap(),
        );

        let client = reqwest::ClientBuilder::new()
            .http2_adaptive_window(true)
            .use_rustls_tls()
            .default_headers(headers)
            .build()
            .unwrap();

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
    pub async fn search(&self, query: &str) -> Result<Vec<TMDbMultiResult>, TMDbErrorResponse> {
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
    pub async fn search_movie(
        &self,
        query: &str,
    ) -> Result<Vec<TMDbMovieResult>, TMDbErrorResponse> {
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
    pub async fn get_movie_details(&self, id: i32) -> Result<TMDbMovieResult, TMDbErrorResponse> {
        let response: TMDbMovieResult = self.request(&format!("movie/{}", id), &[]).await?;

        Ok(response)
    }
}
