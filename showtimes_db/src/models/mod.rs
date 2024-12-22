pub mod common;
pub mod migrations;
pub mod project;
pub mod rss;
pub mod server;
pub mod users;

pub use common::*;
pub use migrations::*;
pub use project::*;
pub use rss::*;
pub use server::*;
pub use users::*;

pub trait ShowModelHandler {
    /// Get the ID
    fn id(&self) -> Option<mongodb::bson::oid::ObjectId>;
    /// Set the ID
    fn set_id(&mut self, id: mongodb::bson::oid::ObjectId);
    /// Unset the ID
    fn unset_id(&mut self);
    /// Get the collection name
    fn collection_name() -> &'static str;
    /// Change the updated time to the current time
    fn updated(&mut self);
}
