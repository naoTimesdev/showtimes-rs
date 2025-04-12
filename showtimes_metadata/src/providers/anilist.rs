//! The provider for Anilist source.
//!
//! This is incomplete and only made to support what Showtimes needed.

use std::str::FromStr;

use serde_json::json;

use crate::{
    errors::{DetailedSerdeError, MetadataResult},
    models::{
        AnilistAiringSchedulePaged, AnilistError, AnilistGraphQLResponseError, AnilistMedia,
        AnilistPageInfo, AnilistPagedData, AnilistResponse, AnilistSingleMedia,
    },
};

const ANILIST_GRAPHQL_URL: &str = "https://graphql.anilist.co/";

#[derive(Debug, Clone)]
/// The rate limit from Anilist.
pub struct AnilistRateLimit {
    /// The maximum of queries you can do per 5 minutes.
    pub limit: u32,
    /// The remaining queries left.
    pub remaining: u32,
    /// The time when the rate limit will reset in seconds since UNIX epoch.
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

/// The main client that provide data from Anilist
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
            "showtimes-rs-metadata/{} (+https://github.com/naoTimesdev/showtimes-rs)",
            env!("CARGO_PKG_VERSION")
        ))
        .expect("Failed to build the User-Agent header for Anilist API");
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(reqwest::header::USER_AGENT, ua_bind);

        let client = reqwest::ClientBuilder::new()
            .http2_adaptive_window(true)
            .default_headers(headers)
            .use_rustls_tls()
            .build()
            .expect("Failed to build reqwest client for Anilist API");

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
    async fn query<T>(
        &mut self,
        query: &str,
        variables: &serde_json::Value,
    ) -> MetadataResult<AnilistResponse<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let json_data = json!({
            "query": query,
            "variables": variables,
        });

        if self.wait_limit && self.rate_limit.exhausted() {
            // Reset is UNIX timestamp
            let wait_time = self.rate_limit.reset - jiff::Timestamp::now().as_second();
            if wait_time > 0 {
                tokio::time::sleep(tokio::time::Duration::from_secs(wait_time as u64)).await;
            }
        }

        let req = self
            .client
            .post(ANILIST_GRAPHQL_URL)
            .json(&json_data)
            .send()
            .await
            .map_err(AnilistError::Request)?;

        let rate_limit: u32 = parse_header_num(req.headers(), "x-ratelimit-limit")?;
        let rate_remaining: u32 = parse_header_num(req.headers(), "x-ratelimit-remaining")?;
        let mut rate_reset: i64 = parse_header_num(req.headers(), "x-ratelimit-reset")?;
        if rate_reset == 0 {
            rate_reset = -1;
        }

        self.rate_limit = AnilistRateLimit {
            limit: rate_limit,
            remaining: rate_remaining,
            reset: rate_reset,
        };

        let status = req.status();
        let headers = req.headers().clone();
        let url = req.url().clone();
        let raw_text = req.text().await.map_err(AnilistError::Request)?;

        // Try parsing as errors
        if let Ok(error) = serde_json::from_str::<AnilistGraphQLResponseError>(&raw_text) {
            return Err(AnilistError::GraphQL(error).into());
        }

        let json_data: AnilistResponse<T> = serde_json::from_str(&raw_text).map_err(|e| {
            AnilistError::Serde(Box::new(DetailedSerdeError::new(
                e, status, &headers, &url, raw_text,
            )))
        })?;

        Ok(json_data)
    }

    /// Search for a media by title
    pub async fn search(&mut self, title: impl Into<String>) -> MetadataResult<Vec<AnilistMedia>> {
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
                    isAdult
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

        let res = self.query::<AnilistPagedData>(queries, &variables).await?;
        if let Some(media) = res.data.page.nodes.media() {
            Ok(media.to_vec())
        } else {
            Ok(vec![])
        }
    }

    /// Get specific media information
    ///
    /// * `id` - The ID of the media
    pub async fn get_media(&mut self, id: i32) -> MetadataResult<AnilistMedia> {
        let queries = r#"query mediaInfo($id:Int) {
            Media(id:$id) {
                id
                idMal
                format
                type
                season
                seasonYear
                episodes
                chapters
                volumes
                isAdult
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
        }"#;

        let variables = json!({
            "id": id,
        });

        let res = self
            .query::<AnilistSingleMedia>(queries, &variables)
            .await?;

        Ok(res.data.media.clone())
    }

    /// Get the airing schedules for a media
    ///
    /// * `id` - The ID of the media
    /// * `page` - The page number
    pub async fn get_airing_schedules(
        &mut self,
        id: i32,
        page: Option<u32>,
    ) -> MetadataResult<AnilistAiringSchedulePaged> {
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
                if p < 1 { 1 } else { p }
            }
            None => 1,
        };

        let variables = json!({
            "id": id,
            "page": act_page,
        });

        let res = self.query::<AnilistPagedData>(queries, &variables).await?;

        if let Some(air_schedules) = res.data.page.nodes.airing_schedules() {
            Ok(AnilistAiringSchedulePaged {
                airing_schedules: air_schedules.to_vec(),
                page_info: res.data.page.page_info,
            })
        } else {
            Ok(AnilistAiringSchedulePaged {
                airing_schedules: vec![],
                page_info: AnilistPageInfo {
                    total: 0,
                    per_page: 50,
                    current_page: 1,
                    has_next_page: false,
                },
            })
        }
    }
}

// T should be u32 or i64
fn parse_header_num<T>(header: &reqwest::header::HeaderMap, name: &str) -> Result<T, AnilistError>
where
    T: Default + FromStr + Copy,
{
    match header.get(name) {
        Some(num) => num
            .to_str()
            .map_err(|_| AnilistError::HeaderToString(name.to_string()))?
            .parse()
            .map_err(|_| AnilistError::StringToNumber(name.to_string())),
        None => Ok(T::default()),
    }
}
