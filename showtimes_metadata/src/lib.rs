// #![warn(missing_docs, clippy::empty_docs, rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../README.md")]

pub mod models;
pub mod providers;

/// Re-export the models module
pub use models as m;
/// The Anilist provider
pub use providers::AnilistProvider;
/// The TMDb provider
pub use providers::TMDbProvider;
/// The VNDB provider
pub use providers::VndbProvider;

/// The provider enum
pub enum Provider {
    /// Anilist provider
    Anilist(AnilistProvider),
    /// TMDb provider
    TMDb(TMDbProvider),
    /// VNDB provider
    VNDB(VndbProvider),
}
