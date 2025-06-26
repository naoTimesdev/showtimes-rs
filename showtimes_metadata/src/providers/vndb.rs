//! The provider for VNDB source.
//!
//! This is incomplete and only made to support what Showtimes needed.

use serde_json::json;

use crate::{
    errors::{DetailedSerdeError, MetadataResult},
    models::{VNDBError, VndbNovel, VndbResult},
};

const VNDB_API_URL: &str = "https://api.vndb.org/kana";
// Common filters used when getting VN data
const VNDB_VN_FILTERS: &str = "id, titles.lang, titles.title, titles.official, titles.latin, titles.main, olang, platforms, image.id, image.url, description, developers.id, developers.name, image.sexual, released";

/// The main client that provide data from VNDB
#[derive(Debug, Clone)]
pub struct VndbProvider {
    client: reqwest::Client,
}

impl VndbProvider {
    /// Create a new VNDB provider
    ///
    /// # Arguments
    /// * `token` - The VNDB API token
    pub fn new(token: impl Into<String>) -> Self {
        let ua_bind = reqwest::header::HeaderValue::from_str(&format!(
            "showtimes-rs-metadata/{} (+https://github.com/naoTimesdev/showtimes-rs)",
            env!("CARGO_PKG_VERSION")
        ))
        .expect("Failed to build the User-Agent header for VNDB API");
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(reqwest::header::USER_AGENT, ua_bind);
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Token {}", token.into()))
                .expect("Failed to build the Auth header for VNDB API"),
        );

        let client = reqwest::ClientBuilder::new()
            .http2_adaptive_window(true)
            .default_headers(headers)
            .use_rustls_tls()
            .build()
            .expect("Failed to build reqwest client for VNDB API");

        VndbProvider { client }
    }

    async fn request(
        &self,
        endpoint: &str,
        params: serde_json::Value,
    ) -> MetadataResult<VndbResult> {
        // json POST
        let req = self
            .client
            .post(format!("{VNDB_API_URL}{endpoint}"))
            .json(&params)
            .send()
            .await
            .map_err(VNDBError::Request)?;

        // is json
        let is_json = match req.headers().get(reqwest::header::CONTENT_TYPE) {
            Some(header) => {
                let header_str = header
                    .to_str()
                    .map_err(|_| VNDBError::HeaderToString("content-type".to_string()))?;

                header_str.starts_with("application/json")
            }
            None => false,
        };

        let status = req.status();
        let headers = req.headers().clone();
        let url = req.url().clone();
        let raw_text = req.text().await.map_err(VNDBError::Request)?;

        if !is_json {
            return Err(VNDBError::Response(raw_text.clone()).into());
        }

        let res = serde_json::from_str::<VndbResult>(&raw_text).map_err(|err| {
            VNDBError::new_serde(DetailedSerdeError::new(
                err, status, &headers, &url, raw_text,
            ))
        })?;

        Ok(res)
    }

    /// Search for a novel by title
    ///
    /// # Arguments
    /// * `title` - The title of the novel
    pub async fn search(&self, title: impl Into<String>) -> MetadataResult<Vec<VndbNovel>> {
        let json_data = json!({
            "filters": ["search","=", title.into()],
            "fields": VNDB_VN_FILTERS
        });

        let res = self.request("/vn", json_data).await?;

        Ok(res.results)
    }

    /// Get novel information by ID
    ///
    /// # Arguments
    /// * `id` - The ID of the novel
    pub async fn get(&self, id: impl Into<String>) -> MetadataResult<VndbNovel> {
        let id: String = id.into();
        if !id.starts_with("v") {
            return Err(VNDBError::InvalidId(id).into());
        }

        // is proper ID?
        let id_test = id.trim_start_matches('v');
        if id_test.parse::<u64>().is_err() {
            return Err(VNDBError::InvalidId(id).into());
        }

        let json_data = json!({
            "filters": ["id","=", id],
            "fields": VNDB_VN_FILTERS
        });

        let res = self.request("/vn", json_data).await?;

        match res.results.first() {
            None => Err(VNDBError::NotFound(id).into()),
            Some(novel) => Ok(novel.clone()),
        }
    }
}
