use futures::TryStreamExt;
use showtimes_db::{
    m::UserKind,
    mongodb::bson::{doc, Document},
    DatabaseShared,
};
use showtimes_shared::ulid::Ulid;

use crate::models::{
    prelude::{PageInfoGQL, PaginatedGQL},
    servers::ServerGQL,
};

/// The config for querying servers
#[derive(Debug, Clone, Copy)]
pub struct ServerQueryUser {
    /// The user ID
    id: Ulid,
    /// The user kind
    kind: UserKind,
}

impl ServerQueryUser {
    /// Create a new server query user
    pub fn new(id: Ulid, kind: UserKind) -> Self {
        ServerQueryUser { id, kind }
    }

    /// Get the user ID
    pub fn id(&self) -> Ulid {
        self.id
    }

    /// Get the user kind
    pub fn kind(&self) -> UserKind {
        self.kind
    }
}

impl From<&showtimes_db::m::User> for ServerQueryUser {
    fn from(user: &showtimes_db::m::User) -> Self {
        ServerQueryUser::new(user.id, user.kind)
    }
}

impl From<showtimes_db::m::User> for ServerQueryUser {
    fn from(user: showtimes_db::m::User) -> Self {
        ServerQueryUser::new(user.id, user.kind)
    }
}

/// The config for querying servers
#[derive(Debug, Clone, Default)]
pub struct ServerQuery {
    /// Specify server IDs to query
    ids: Option<Vec<Ulid>>,
    /// The number of servers to return
    per_page: Option<u32>,
    /// The cursor to start from
    cursor: Option<Ulid>,
    /// The current user fetching this data
    current_user: Option<ServerQueryUser>,
    /// Disable project fetch
    disable_projects: bool,
}

impl ServerQuery {
    /// Create a new server query
    pub fn new() -> Self {
        ServerQuery::default()
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

    /// Disable project fetch
    pub fn with_disable_projects(mut self) -> Self {
        self.disable_projects = true;
        self
    }

    pub fn set_disable_projects(&mut self) {
        self.disable_projects = true;
    }
}

pub async fn query_servers_paginated(
    ctx: &async_graphql::Context<'_>,
    queries: ServerQuery,
) -> async_graphql::Result<PaginatedGQL<ServerGQL>> {
    let db = ctx.data_unchecked::<DatabaseShared>();

    // Allowed range of per_page is 10-100, with
    let per_page = queries
        .per_page
        .filter(|&p| (2..=100).contains(&p))
        .unwrap_or(20);

    let srv_handler = showtimes_db::ServerHandler::new(db);

    let mut doc_query = match (queries.cursor, queries.ids) {
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

    let mut count_query = Document::new();

    if let Some(user) = &queries.current_user {
        if user.kind == UserKind::User {
            doc_query.insert("owners.id", user.id.to_string());
            count_query.insert("owners.id", user.id.to_string());
        }
    }

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

    let mut all_servers: Vec<showtimes_db::m::Server> = cursor.try_collect().await?;

    // If all_servers is equal to per_page, then there is a next page
    let last_srv = if all_servers.len() > per_page as usize {
        Some(all_servers.pop().unwrap())
    } else {
        None
    };

    let page_info = PageInfoGQL::new(count, per_page, last_srv.map(|p| p.id.into()));

    Ok(PaginatedGQL::new(
        all_servers
            .into_iter()
            .map(|p| {
                let srv_gql: ServerGQL = p.into();
                let srv_gql = if queries.disable_projects {
                    srv_gql.with_projects_disabled()
                } else {
                    srv_gql
                };

                if let Some(user) = &queries.current_user {
                    match user.kind {
                        UserKind::User => srv_gql.with_current_user(user.id),
                        _ => srv_gql,
                    }
                } else {
                    srv_gql
                }
            })
            .collect(),
        page_info,
    ))
}
