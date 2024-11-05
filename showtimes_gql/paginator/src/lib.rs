#![warn(missing_docs, clippy::empty_docs, rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../../README.md")]

use std::fmt::Debug;

use showtimes_gql_common::PageInfoGQL;

pub mod projects;
pub mod servers;
pub mod users;

/// A paginated data structure
pub struct PaginatedResult<
    T: Debug + Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
> {
    /// The items list
    pub(crate) nodes: Vec<T>,
    /// The page information
    pub(crate) page_info: PageInfoGQL,
}

impl<T: Debug + Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned>
    PaginatedResult<T>
{
    /// Create a new PaginatedResult
    pub fn new(nodes: Vec<T>, page_info: PageInfoGQL) -> Self {
        PaginatedResult { nodes, page_info }
    }

    /// Get the nodes
    pub fn nodes(&self) -> &[T] {
        &self.nodes
    }

    /// Get the page information
    pub fn page_info(&self) -> &PageInfoGQL {
        &self.page_info
    }
}

impl<T: Debug + Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned> Default
    for PaginatedResult<T>
{
    fn default() -> Self {
        PaginatedResult::new(Vec::new(), PageInfoGQL::default())
    }
}
