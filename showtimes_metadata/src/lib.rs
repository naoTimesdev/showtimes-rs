pub mod models;
pub mod providers;

/// Re-export the models module
pub use models as m;
/// The Anilist provider
pub use providers::AnilistProvider;
/// The VNDB provider
pub use providers::VndbProvider;
