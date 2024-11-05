//! Some common shared models

use async_graphql::{OutputType, SimpleObject};
use showtimes_gql_common::PageInfoGQL;

use super::{projects::ProjectGQL, servers::ServerGQL, users::UserGQL};

/// A paginated data structure
#[derive(SimpleObject)]
#[graphql(concrete(name = "ProjectPaginatedGQL", params(ProjectGQL)))]
#[graphql(concrete(name = "ServerPaginatedGQL", params(ServerGQL)))]
#[graphql(concrete(name = "UserPaginatedGQL", params(UserGQL)))]
pub struct PaginatedGQL<T: OutputType> {
    /// The items list
    nodes: Vec<T>,
    /// The page information
    #[graphql(name = "pageInfo")]
    page_info: PageInfoGQL,
}

impl<T: OutputType> PaginatedGQL<T> {
    /// Create a new PaginatedGQL
    pub fn new(nodes: Vec<T>, page_info: PageInfoGQL) -> Self {
        PaginatedGQL { nodes, page_info }
    }
}

impl<T: OutputType> Default for PaginatedGQL<T> {
    fn default() -> Self {
        PaginatedGQL {
            nodes: Vec::new(),
            page_info: PageInfoGQL::default(),
        }
    }
}
