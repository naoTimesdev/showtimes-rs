use serde_json::json;

use crate::models::{VndbNovel, VndbResult};

const VNDB_API_URL: &str = "https://api.vndb.org/kana";

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
            "showtimes-rs-ext/{}",
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
            reqwest::header::HeaderValue::from_str(&format!("Token {}", token.into())).unwrap(),
        );

        let client = reqwest::ClientBuilder::new()
            .http2_adaptive_window(true)
            .use_rustls_tls()
            .default_headers(headers)
            .build()
            .unwrap();

        VndbProvider { client }
    }

    /// Search for a novel by title
    ///
    /// # Arguments
    /// * `title` - The title of the novel
    pub async fn search(&self, title: impl Into<String>) -> anyhow::Result<Vec<VndbNovel>> {
        let json_data = json!({
            "filters": ["search","=", title.into()],
            "fields": "id, titles.lang, titles.title, titles.official, titles.main, olang, platforms, image.id, image.url, image.dims, description, developers.id, developers.name, image.sexual, released"
        });

        // json POST
        let req = self
            .client
            .post(format!("{}/vn", VNDB_API_URL))
            .json(&json_data)
            .send()
            .await?;

        let res = req.json::<VndbResult>().await?;

        Ok(res.results)
    }

    /// Get novel information by ID
    ///
    /// # Arguments
    /// * `id` - The ID of the novel
    pub async fn get(&self, id: impl Into<String>) -> anyhow::Result<VndbNovel> {
        let id: String = id.into();
        if !id.starts_with("v") {
            anyhow::bail!("Invalid VNDB novel ID");
        }

        // is proper ID?
        let id_test = id.trim_start_matches('v');
        if id_test.parse::<u64>().is_err() {
            anyhow::bail!("Invalid VNDB novel ID");
        }

        let json_data = json!({
            "filters": ["id","=", id],
            "fields": "id, titles.lang, titles.title, titles.official, titles.main, olang, platforms, image.id, image.url, image.dims, description, developers.id, developers.name, image.sexual, released"
        });

        // json POST
        let req = self
            .client
            .post(format!("{}/vn", VNDB_API_URL))
            .json(&json_data)
            .send()
            .await?;

        let res = req.json::<VndbResult>().await?;

        if res.results.is_empty() {
            anyhow::bail!("VNDB novel not found");
        }

        Ok(res.results.into_iter().next().unwrap())
    }
}
