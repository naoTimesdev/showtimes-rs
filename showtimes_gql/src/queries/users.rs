use futures::TryStreamExt;
use showtimes_db::{
    mongodb::bson::{doc, Document},
    DatabaseShared,
};
use showtimes_shared::ulid::Ulid;

use super::ServerQueryUser;
use crate::models::{
    prelude::{PageInfoGQL, PaginatedGQL},
    users::UserGQL,
};

/// The config for querying servers
#[derive(Debug, Clone, Default)]
pub struct UserQuery {
    /// Specify server IDs to query
    ids: Option<Vec<Ulid>>,
    /// The number of servers to return
    per_page: Option<u32>,
    /// The cursor to start from
    cursor: Option<Ulid>,
    /// Current user
    current_user: Option<ServerQueryUser>,
}

impl UserQuery {
    /// Create a new server query
    pub fn new() -> Self {
        UserQuery::default()
    }

    /// Set the IDs to query
    pub fn with_ids(mut self, ids: Vec<Ulid>) -> Self {
        self.ids = Some(ids);
        self
    }

    pub fn set_ids(&mut self, ids: Vec<Ulid>) {
        self.ids = Some(ids);
    }

    /// Set the number of servers to return
    pub fn with_per_page(mut self, per_page: u32) -> Self {
        self.per_page = Some(per_page);
        self
    }

    pub fn set_per_page(&mut self, per_page: u32) {
        self.per_page = Some(per_page);
    }

    /// Set the cursor to start from
    pub fn with_cursor(mut self, cursor: Ulid) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub fn set_cursor(&mut self, cursor: Ulid) {
        self.cursor = Some(cursor);
    }

    /// Set the current user fetching this data
    pub fn with_current_user(mut self, user: ServerQueryUser) -> Self {
        self.current_user = Some(user);
        self
    }

    pub fn set_current_user(&mut self, user: ServerQueryUser) {
        self.current_user = Some(user);
    }
}

pub async fn query_users_paginated(
    ctx: &async_graphql::Context<'_>,
    queries: UserQuery,
) -> async_graphql::Result<PaginatedGQL<UserGQL>> {
    let db = ctx.data_unchecked::<DatabaseShared>();

    // Allowed range of per_page is 10-100, with
    let per_page = queries
        .per_page
        .filter(|&p| (2..=100).contains(&p))
        .unwrap_or(20);

    let srv_handler = showtimes_db::UserHandler::new(db);

    let doc_query = match (queries.cursor, &queries.ids) {
        (Some(cursor), Some(ids)) => {
            let ids: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();
            doc! {
                "id": { "$gte": cursor.to_string(), "$in": ids }
            }
        }
        (Some(cursor), None) => {
            doc! {
                "id": { "$gte": cursor.to_string() }
            }
        }
        (None, Some(ids)) => {
            let ids: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();
            doc! {
                "id": { "$in": ids }
            }
        }
        (None, None) => Document::new(),
    };

    let count_query = match &queries.ids {
        Some(ids) => {
            let ids: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();
            doc! {
                "id": { "$in": ids }
            }
        }
        None => Document::new(),
    };

    let cursor = srv_handler
        .get_collection()
        .find(doc_query)
        .limit((per_page + 1) as i64)
        .sort(doc! { "id": 1 })
        .await?;
    let count = srv_handler
        .get_collection()
        .count_documents(count_query)
        .await?;

    let mut all_users: Vec<showtimes_db::m::User> = cursor.try_collect().await?;

    // If all_users is equal to per_page, then there is a next page
    let last_srv = if all_users.len() > per_page as usize {
        Some(all_users.pop().unwrap())
    } else {
        None
    };

    let page_info = PageInfoGQL::new(count, per_page, last_srv.map(|p| p.id.into()));

    Ok(PaginatedGQL::new(
        all_users
            .into_iter()
            .map(|p| {
                let usr_gql = UserGQL::from(p);
                if let Some(user) = &queries.current_user {
                    usr_gql.with_requester(user.clone())
                } else {
                    usr_gql
                }
            })
            .collect(),
        page_info,
    ))
}
