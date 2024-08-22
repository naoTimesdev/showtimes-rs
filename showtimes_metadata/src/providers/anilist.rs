use serde_json::json;

use crate::models::{AnilistAiringSchedulePaged, AnilistMedia, AnilistResponse};

const ANILIST_GRAPHQL_URL: &str = "https://graphql.anilist.co/";

#[derive(Debug, Clone)]
pub struct AnilistRateLimit {
    pub limit: u32,
    pub remaining: u32,
    pub reset: i64,
}

impl AnilistRateLimit {
    /// Check if the rate limit has been reached
    pub fn exhausted(&self) -> bool {
        self.remaining == 0
    }
}

impl Default for AnilistRateLimit {
    fn default() -> Self {
        AnilistRateLimit {
            limit: 90,
            remaining: 90,
            reset: -1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnilistProvider {
    client: reqwest::Client,
    rate_limit: AnilistRateLimit,
    wait_limit: bool,
}

impl AnilistProvider {
    /// Create a new Anilist provider
    ///
    /// * `wait_limit` - Whether to wait for the rate limit to reset
    pub fn new(wait_limit: bool) -> Self {
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

        let client = reqwest::ClientBuilder::new()
            .http2_adaptive_window(true)
            .use_rustls_tls()
            .default_headers(headers)
            .build()
            .unwrap();

        AnilistProvider {
            client,
            rate_limit: AnilistRateLimit::default(),
            wait_limit,
        }
    }

    /// Get the current rate limit
    pub fn rate_limit(&self) -> &AnilistRateLimit {
        &self.rate_limit
    }

    /// Check if we do wait for rate limit to reset
    pub fn is_wait(&self) -> bool {
        self.wait_limit
    }

    /// Set the current wait limit
    pub fn set_wait_limit(&mut self, wait_limit: bool) {
        self.wait_limit = wait_limit;
    }

    /// Do a GraphQL query
    ///
    /// * `query` - The query to send
    /// * `variables` - The variables to send
    async fn query(
        &mut self,
        query: &str,
        variables: &serde_json::Value,
    ) -> anyhow::Result<AnilistResponse> {
        let json_data = json!({
            "query": query,
            "variables": variables,
        });

        if self.wait_limit && self.rate_limit.exhausted() {
            // Reset is UNIX timestamp
            let wait_time = self.rate_limit.reset - chrono::Utc::now().timestamp();
            if wait_time > 0 {
                tokio::time::sleep(tokio::time::Duration::from_secs(wait_time as u64)).await;
            }
        }

        let req = self
            .client
            .post(ANILIST_GRAPHQL_URL)
            .json(&json_data)
            .send()
            .await?;

        let rate_limit = match req.headers().get("x-ratelimit-limit") {
            Some(limit) => limit.to_str().unwrap().parse().unwrap(),
            None => 0u32,
        };
        let rate_remaining = match req.headers().get("x-ratelimit-remaining") {
            Some(remaining) => remaining.to_str().unwrap().parse().unwrap(),
            None => 0u32,
        };
        let rate_reset: i64 = match req.headers().get("x-ratelimit-reset") {
            Some(reset) => reset.to_str().unwrap().parse().unwrap(),
            None => -1i64,
        };

        self.rate_limit = AnilistRateLimit {
            limit: rate_limit,
            remaining: rate_remaining,
            reset: rate_reset,
        };

        let json_data: AnilistResponse = req.json().await?;

        Ok(json_data)
    }

    /// Search for a media by title
    pub async fn search(&mut self, title: impl Into<String>) -> anyhow::Result<Vec<AnilistMedia>> {
        let queries = r#"query mediaSearch($search:String) {
            Page (page:1,perPage:25) {
                media(search:$search,sort:[SEARCH_MATCH]) {
                    id
                    idMal
                    format
                    type
                    season
                    seasonYear
                    episodes
                    chapters
                    volumes
                    description(asHtml:false)
                    status(version:2)
                    startDate {
                        year
                        month
                        day
                    }
                    title {
                        romaji
                        native
                        english
                    }
                    coverImage {
                        medium
                        large
                        extraLarge
                    }
                }
                pageInfo {
                    total
                    perPage
                    currentPage
                    hasNextPage
                }
            }
        }"#;

        let variables = json!({
            "search": title.into(),
        });

        let res = self.query(queries, &variables).await?;
        let media_data = res.data.page.nodes.media().unwrap();

        Ok(media_data.clone())
    }

    /// Get the airing schedules for a media
    ///
    /// * `id` - The ID of the media
    /// * `page` - The page number
    pub async fn get_airing_schedules(
        &mut self,
        id: i32,
        page: Option<u32>,
    ) -> anyhow::Result<AnilistAiringSchedulePaged> {
        let queries = r#"query mediaSchedule($id:Int,$page:Int!) {
            Page (page:$page,perPage:50) {
                airingSchedules(mediaId:$id,sort:[EPISODE]) {
                    id
                    episode
                    airingAt
                    mediaId
                }
                pageInfo {
                    total
                    perPage
                    currentPage
                    lastPage
                    hasNextPage
                }
            }
        }"#;

        let act_page = match page {
            Some(p) => {
                // If page is less than 1, set it to 1
                if p < 1 {
                    1
                } else {
                    p
                }
            }
            None => 1,
        };

        let variables = json!({
            "id": id,
            "page": act_page,
        });

        let res = self.query(queries, &variables).await?;
        let air_schedules = res.data.page.nodes.airing_schedules().unwrap();
        let page_info = res.data.page.page_info.clone();

        Ok(AnilistAiringSchedulePaged {
            airing_schedules: air_schedules.clone(),
            page_info,
        })
    }
}
