//! A type definition for the VNDB v2 API
//!
//! This is incomplete and only made to support what Showtimes needed.

use serde::{Deserialize, Serialize};

/// The VNDB title list information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VndbTitle {
    /// The title of the VN
    pub title: String,
    /// The latin/romanized title of the VN
    pub latin: Option<String>,
    /// The language of the title
    pub lang: String,
    /// Is this official translation of the title?
    pub official: bool,
    /// Is this title what VNDB use as main title?
    pub main: bool,
}

/// The VNDB developer information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VndbDeveloper {
    /// The developer ID
    pub id: String,
    /// The developer name
    pub name: String,
}

/// The VNDB image information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VndbImage {
    /// The image ID
    pub id: String,
    /// The image URL
    pub url: String,
    /// The image NSFW flag
    ///
    /// 0 = Safe
    /// 1 = Tame
    /// 2 = Explicit
    #[serde(rename = "sexual")]
    pub nsfw: f32,
}

impl VndbImage {
    /// Is the image NSFW
    pub fn is_nsfw(&self) -> bool {
        self.nsfw > 1.5
    }
}

/// The VNDB novel information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VndbNovel {
    /// The novel ID
    pub id: String,
    /// The novel titles
    pub titles: Vec<VndbTitle>,
    /// The novel image
    pub image: VndbImage,
    /// The novel description
    pub description: Option<String>,
    /// The novel original language
    #[serde(rename = "olang")]
    pub original_lang: String,
    /// The novel release date in original country/language
    ///
    /// YYYY-MM-DD
    pub released: Option<String>,
    /// The novel developers
    pub developers: Vec<VndbDeveloper>,
    /// The novel released platforms
    pub platforms: Vec<String>,
}

impl VndbNovel {
    /// Get the main title of the novel
    ///
    /// While this return with [`Option`], it should always return with a value
    pub fn get_main_title(&self) -> Option<String> {
        self.titles.iter().find(|t| t.main).map(|t| t.title.clone())
    }

    /// Get "best" english title of the novel
    pub fn get_english_title(&self) -> Option<String> {
        let mut find_en_title: Vec<&VndbTitle> =
            self.titles.iter().filter(|&t| t.lang == "en").collect();

        find_en_title.sort_by(sort_vndb_title);
        find_en_title.first().map(|t| t.title.clone())
    }

    /// Get the "best" original title of the novel
    pub fn get_original_title(&self) -> Option<VndbTitle> {
        let mut find_original_title: Vec<&VndbTitle> = self
            .titles
            .iter()
            .filter(|&t| t.lang == self.original_lang)
            .collect();

        find_original_title.sort_by(sort_vndb_title);
        find_original_title.first().map(|&t| t.clone())
    }

    /// Get the release date
    ///
    /// If "TBA", "TBD", "Unknown", "Today" or "Now", it will return with `None`
    pub fn get_release_date(&self) -> Option<String> {
        self.released.as_ref().and_then(|d| {
            if d.eq_ignore_ascii_case("TBA")
                || d.eq_ignore_ascii_case("TBD")
                || d.eq_ignore_ascii_case("Unknown")
                || d.eq_ignore_ascii_case("Today")
                || d.eq_ignore_ascii_case("Now")
            {
                None
            } else {
                Some(d.clone())
            }
        })
    }
}

/// The VNDB API results
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VndbResult {
    /// The results itself
    pub results: Vec<VndbNovel>,
    /// Is there more results?
    pub more: bool,
}

fn sort_vndb_title(a: &&VndbTitle, b: &&VndbTitle) -> std::cmp::Ordering {
    if a.main && !b.main {
        std::cmp::Ordering::Less
    } else if !a.main && b.main {
        std::cmp::Ordering::Greater
    } else if a.official && !b.official {
        std::cmp::Ordering::Less
    } else if !a.official && b.official {
        std::cmp::Ordering::Greater
    } else {
        std::cmp::Ordering::Equal
    }
}
