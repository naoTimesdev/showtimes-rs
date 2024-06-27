use serde::{Deserialize, Serialize};

/// The VNDB title list information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VndbTitle {
    /// The title of the VN
    pub title: String,
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
    pub nsfw: u8,
}

impl VndbImage {
    /// Is the image NSFW
    pub fn is_nsfw(&self) -> bool {
        self.nsfw > 0
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
    pub description: String,
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

    /// Get english title of the novel
    pub fn get_english_title(&self) -> Option<String> {
        self.titles
            .iter()
            .find(|t| t.lang == "en")
            .map(|t| t.title.clone())
    }
}
