use std::sync::Arc;
use tokio::sync::Mutex;

use async_graphql::{Enum, Object, SimpleObject};
use showtimes_metadata::{
    m::{AnilistFuzzyDate, AnilistMedia, AnilistMediaFormat, TMDbMultiResult, VndbNovel},
    AnilistProvider, TMDbProvider, VndbProvider,
};

use super::projects::ProjectTypeGQL;

type AnilistProviderShared = Arc<Mutex<AnilistProvider>>;
type TMDbProviderShared = Arc<TMDbProvider>;
type VNDBProviderShared = Arc<VndbProvider>;

/// The preferred title to use for the external search
#[derive(Enum, Debug, Copy, Clone, Eq, PartialEq, Default)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum ExternalSearchTitlePrefer {
    #[default]
    English,
    Native,
    Romanized,
}

/// The source of the external metadata search
#[derive(Enum, Debug, Copy, Clone, Eq, PartialEq)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum ExternalSearchSource {
    Anilist,
    #[graphql(name = "TMDB")]
    TMDb,
    VNDB,
}

/// A "fuzzy"-date, where we might not have all the information
/// to create a full date.
#[derive(SimpleObject)]
pub struct ExternalSearchFuzzyDate {
    /// The year of the date
    year: Option<i32>,
    /// The month of the date
    month: Option<i32>,
    /// The day of the date
    day: Option<i32>,
}

/// The resulting title from a external metadata search
#[derive(SimpleObject)]
pub struct ExternalSearchTitle {
    /// The english/translated title
    english: Option<String>,
    /// The native title
    native: Option<String>,
    /// The romanized title
    romanized: Option<String>,
}

#[derive(SimpleObject)]
pub struct ExternalSearch {
    /// The identifier from the external service
    id: String,
    /// The selected title of media.
    ///
    /// By default, the english title is selected. This can be changed by setting the `title` field.
    /// when querying the external search. If not set, the english title will be selected.
    title: String,
    /// All the titles of the media
    titles: ExternalSearchTitle,
    /// The format of the media
    format: ProjectTypeGQL,
    /// The description of the media
    description: Option<String>,
    /// The release date of the media
    release_date: Option<ExternalSearchFuzzyDate>,
    /// The image URL of the media
    image: Option<String>,
    /// The total "episodes" of the media
    ///
    /// This is only used if `kind` has multiple episodes format.
    episodes: Option<u32>,
    /// The source of the media
    source: ExternalSearchSource,
    /// If the media is adult or NSFW
    nsfw: bool,
}

impl ExternalSearch {
    /// Convert from an [`AnilistMedia`] to an [`ExternalSearch`]
    pub fn from_anilist(media: &AnilistMedia, prefer_title: ExternalSearchTitlePrefer) -> Self {
        let title = get_title(
            media.title.english.clone(),
            media.title.native.clone(),
            media.title.romaji.clone(),
            prefer_title,
        )
        .unwrap_or_default();

        Self {
            id: media.id.to_string(),
            title,
            titles: ExternalSearchTitle {
                english: media.title.english.clone(),
                native: media.title.native.clone(),
                romanized: media.title.romaji.clone(),
            },
            format: media.format.into(),
            description: media.description.clone(),
            release_date: media.start_date.map(|d| d.into()),
            image: media.cover_image.get_image(),
            episodes: media.episodes.map(|e| e as u32),
            source: ExternalSearchSource::Anilist,
            nsfw: media.is_adult,
        }
    }

    /// Convert from a [`TMDbMultiResult`] to an [`ExternalSearch`]
    pub fn from_tmdb(media: &TMDbMultiResult, prefer_title: ExternalSearchTitlePrefer) -> Self {
        let title = get_title(
            Some(media.title()),
            media.original_title(),
            None,
            prefer_title,
        )
        .unwrap_or_default();

        // release_date is YYYY-MM-DD, but can be YYYY-MM or YYYY
        let release_date = media.release_date().and_then(|d| yyyy_mm_dd_to_fuzzy(&d));

        let format = match media.media_type {
            showtimes_metadata::m::TMDbMediaType::Tv => ProjectTypeGQL::Series,
            showtimes_metadata::m::TMDbMediaType::Movie => ProjectTypeGQL::Movies,
            _ => ProjectTypeGQL::Unknown,
        };

        Self {
            id: media.id.to_string(),
            title,
            titles: ExternalSearchTitle {
                english: Some(media.title()),
                native: None,
                romanized: media.original_title(),
            },
            format,
            description: media.overview.clone(),
            release_date,
            image: media.poster_url(),
            episodes: None,
            source: ExternalSearchSource::TMDb,
            nsfw: media.adult,
        }
    }

    pub fn from_vndb(media: &VndbNovel, prefer_title: ExternalSearchTitlePrefer) -> Self {
        let en_title = media.get_english_title();
        let original_title = media.get_original_title();
        let native_title = original_title.clone().map(|t| t.title.clone());
        let romaji_title = original_title.map(|t| t.latin.clone()).unwrap_or_default();

        let title = get_title(
            en_title.clone(),
            native_title.clone(),
            romaji_title.clone(),
            prefer_title,
        )
        .unwrap_or_default();

        let rls_date = media
            .get_release_date()
            .and_then(|d| yyyy_mm_dd_to_fuzzy(&d));

        Self {
            id: media.id.clone(),
            title,
            titles: ExternalSearchTitle {
                english: en_title,
                native: native_title,
                romanized: romaji_title,
            },
            format: ProjectTypeGQL::VisualNovel,
            description: media.description.clone(),
            release_date: rls_date,
            image: Some(media.image.url.clone()),
            episodes: None,
            source: ExternalSearchSource::VNDB,
            // Bad metrics but it kinda works I guess?
            nsfw: media.image.is_nsfw(),
        }
    }
}

/// The results of the external search
type ExternalSearchResults = Vec<ExternalSearch>;

/// Get the title based on the prefer
fn get_title(
    english: Option<String>,
    native: Option<String>,
    romaji: Option<String>,
    prefer: ExternalSearchTitlePrefer,
) -> Option<String> {
    match prefer {
        ExternalSearchTitlePrefer::English => english
            .or_else(|| native.clone())
            .or_else(|| romaji.clone()),
        ExternalSearchTitlePrefer::Native => native
            .or_else(|| english.clone())
            .or_else(|| romaji.clone()),
        ExternalSearchTitlePrefer::Romanized => romaji
            .or_else(|| english.clone())
            .or_else(|| native.clone()),
    }
}

fn yyyy_mm_dd_to_fuzzy(date: &str) -> Option<ExternalSearchFuzzyDate> {
    let parts: Vec<&str> = date.split('-').collect();
    let year = parts.first().and_then(|y| y.parse().ok());
    let month = parts.get(1).and_then(|m| m.parse().ok());
    let day = parts.get(2).and_then(|d| d.parse().ok());

    // If all None, return None
    if year.is_none() && month.is_none() && day.is_none() {
        None
    } else {
        Some(ExternalSearchFuzzyDate { year, month, day })
    }
}

impl From<AnilistMediaFormat> for ProjectTypeGQL {
    fn from(value: AnilistMediaFormat) -> Self {
        match value {
            AnilistMediaFormat::Tv | AnilistMediaFormat::TvShort | AnilistMediaFormat::ONA => {
                ProjectTypeGQL::Series
            }
            AnilistMediaFormat::Movie => ProjectTypeGQL::Movies,
            AnilistMediaFormat::Special => ProjectTypeGQL::OVAs,
            AnilistMediaFormat::OVA => ProjectTypeGQL::OVAs,
            AnilistMediaFormat::Music => ProjectTypeGQL::OVAs,
            AnilistMediaFormat::Manga => ProjectTypeGQL::Manga,
            AnilistMediaFormat::Novel => ProjectTypeGQL::LightNovel,
            AnilistMediaFormat::OneShot => ProjectTypeGQL::Manga,
        }
    }
}

impl From<AnilistFuzzyDate> for ExternalSearchFuzzyDate {
    fn from(value: AnilistFuzzyDate) -> Self {
        Self {
            year: value.year,
            month: value.month,
            day: value.day,
        }
    }
}

/// Do a search on external source or internal database
pub struct QuerySearchRoot;

impl QuerySearchRoot {
    pub fn new() -> Self {
        Self
    }
}

#[Object]
impl QuerySearchRoot {
    /// Search for media from Anilist
    async fn anilist(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The query to search for")] query: String,
        #[graphql(desc = "The prefer title to use, default to ENGLISH")] prefer_title: Option<
            ExternalSearchTitlePrefer,
        >,
    ) -> async_graphql::Result<ExternalSearchResults> {
        let prefer_title = prefer_title.unwrap_or(ExternalSearchTitlePrefer::English);
        let provider = ctx.data_unchecked::<AnilistProviderShared>();
        let mut query_server = provider.lock().await;
        let results = query_server.search(query).await?;

        Ok(results
            .iter()
            .map(|m| ExternalSearch::from_anilist(m, prefer_title))
            .collect())
    }

    /// Search for media from TMDb
    ///
    /// This metadata provider is optional, so this might just return an empty list.
    async fn tmdb(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The query to search for")] query: String,
        #[graphql(desc = "The prefer title to use, default to ENGLISH")] prefer_title: Option<
            ExternalSearchTitlePrefer,
        >,
    ) -> async_graphql::Result<ExternalSearchResults> {
        let prefer_title = prefer_title.unwrap_or(ExternalSearchTitlePrefer::English);
        let provider = ctx.data_opt::<TMDbProviderShared>();

        // TMDb provider is optional
        match provider {
            Some(provider) => {
                let results = provider.search(&query).await?;

                Ok(results
                    .iter()
                    // Only allow movies and tv shows
                    .filter_map(|m| match m.media_type {
                        showtimes_metadata::m::TMDbMediaType::Movie
                        | showtimes_metadata::m::TMDbMediaType::Tv => {
                            Some(ExternalSearch::from_tmdb(m, prefer_title))
                        }
                        _ => None,
                    })
                    .collect())
            }
            None => Ok(vec![]),
        }
    }

    /// Search for media from VNDB
    ///
    /// This metadata provider is optional, so this might just return an empty list.
    async fn vndb(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The query to search for")] query: String,
        #[graphql(desc = "The prefer title to use, default to ENGLISH")] prefer_title: Option<
            ExternalSearchTitlePrefer,
        >,
    ) -> async_graphql::Result<ExternalSearchResults> {
        let prefer_title = prefer_title.unwrap_or(ExternalSearchTitlePrefer::English);

        // VNDB provider is optional
        match ctx.data_opt::<VNDBProviderShared>() {
            Some(provider) => {
                let results = provider.search(query).await?;

                Ok(results
                    .iter()
                    .map(|m| ExternalSearch::from_vndb(m, prefer_title))
                    .collect())
            }
            None => Ok(vec![]),
        }
    }
}