//! Project related queries

use std::collections::HashMap;

use futures_util::TryStreamExt;
use showtimes_db::{
    m::UserPrivilege,
    mongodb::bson::{doc, Document},
    DatabaseShared,
};
use showtimes_gql_common::{
    errors::GQLError,
    queries::{MinimalServerUsers, ServerQueryUser},
    GQLDataLoaderWhere, GQLErrorCode, GQLErrorExt, PageInfoGQL, SortOrderGQL,
};
use showtimes_shared::ulid::Ulid;

use crate::PaginatedResult;

/// The config for querying servers
#[derive(Debug, Clone, Default)]
pub struct ProjectQuery {
    /// Specify project IDs to query
    ids: Option<Vec<Ulid>>,
    /// The number of servers to return
    per_page: Option<u32>,
    /// The cursor to start from
    cursor: Option<Ulid>,
    /// Sort order
    sort: SortOrderGQL,
    /// The server that created the project
    creators: Option<Vec<Ulid>>,
    /// Allowed servers to query
    servers_users: Option<Vec<MinimalServerUsers>>,
    current_user: Option<ServerQueryUser>,
    unpaged: bool,
}

#[allow(dead_code)]
impl ProjectQuery {
    /// Create a new server query
    pub fn new() -> Self {
        ProjectQuery::default()
    }

    /// Set the IDs to query
    pub fn with_ids(mut self, ids: Vec<Ulid>) -> Self {
        self.ids = Some(ids);
        self
    }

    /// Set the IDs to query
    pub fn set_ids(&mut self, ids: Vec<Ulid>) {
        self.ids = Some(ids);
    }

    /// Set the number of servers to return
    pub fn with_per_page(mut self, per_page: u32) -> Self {
        self.per_page = Some(per_page);
        self
    }

    /// Set the number of servers to return
    pub fn set_per_page(&mut self, per_page: u32) {
        self.per_page = Some(per_page);
    }

    /// Set the cursor to start from
    pub fn with_cursor(mut self, cursor: Ulid) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Set the cursor to start from
    pub fn set_cursor(&mut self, cursor: Ulid) {
        self.cursor = Some(cursor);
    }

    /// Set the sort order
    pub fn set_sort(&mut self, sort: SortOrderGQL) {
        self.sort = sort;
    }

    /// Set the servers fetching this data
    pub fn with_creators(mut self, user: Vec<Ulid>) -> Self {
        self.creators = Some(user);
        self
    }

    /// Set the servers fetching this data
    pub fn set_creators(&mut self, user: &[Ulid]) {
        self.creators = Some(user.to_vec());
    }

    /// Set the allowed servers to query
    pub fn with_allowed_servers(mut self, servers: Vec<showtimes_db::m::Server>) -> Self {
        self.servers_users = Some(servers.into_iter().map(|s| s.into()).collect());
        self
    }

    /// Set the allowed servers to query
    pub fn set_allowed_servers(&mut self, servers: Vec<showtimes_db::m::Server>) {
        self.servers_users = Some(servers.into_iter().map(|s| s.into()).collect());
    }

    /// Set the allowed servers to query
    pub fn with_allowed_servers_minimal(mut self, servers: Vec<MinimalServerUsers>) -> Self {
        self.servers_users = Some(servers);
        self
    }

    /// Set the allowed servers to query
    pub fn set_allowed_servers_minimal(&mut self, servers: Vec<MinimalServerUsers>) {
        self.servers_users = Some(servers);
    }

    /// Set the current user fetching this data
    pub fn with_current_user(mut self, user: ServerQueryUser) -> Self {
        self.current_user = Some(user);
        self
    }

    /// Set the current user fetching this data
    pub fn set_current_user(&mut self, user: ServerQueryUser) {
        self.current_user = Some(user);
    }

    /// Do unpaged query
    pub fn with_unpaged(mut self) -> Self {
        self.unpaged = true;
        self
    }

    /// Set unpaged query flag
    pub fn set_unpaged(&mut self) {
        self.unpaged = true;
    }

    fn dump_query(&self, ctx: &mut async_graphql::ErrorExtensionValues) {
        if let Some(ids) = &self.ids {
            ctx.set(
                "ids",
                ids.iter().map(|id| id.to_string()).collect::<Vec<String>>(),
            );
        }
        if let Some(per_page) = self.per_page {
            ctx.set("per_page", per_page);
        }
        if let Some(cursor) = &self.cursor {
            ctx.set("cursor", cursor.to_string());
        }
        ctx.set("sort", self.sort.to_name());
        if let Some(creators) = &self.creators {
            ctx.set(
                "creators",
                creators
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<String>>(),
            );
        }
        if let Some(servers) = &self.servers_users {
            let mapped_data = servers
                .iter()
                .map(|s| s.as_graphql_value())
                .collect::<Vec<_>>();

            ctx.set("servers", mapped_data);
        }
        if let Some(user) = &self.current_user {
            ctx.set("current_user", user.as_graphql_value());
        }
        ctx.set("unpaged", self.unpaged);
    }
}

/// Query the projects database and return the paginated data.
pub async fn query_projects_paginated(
    ctx: &async_graphql::Context<'_>,
    queries: ProjectQuery,
) -> async_graphql::Result<PaginatedResult<showtimes_db::m::Project>> {
    let db = ctx.data_unchecked::<DatabaseShared>();

    // Allowed range of per_page is 10-100, with
    let per_page = queries
        .per_page
        .filter(|&p| (2..=100).contains(&p))
        .unwrap_or(20);

    let prj_handler = showtimes_db::ProjectHandler::new(db);

    let fetch_docs = match (&queries.servers_users, queries.current_user) {
        (Some(servers), Some(user_info)) => {
            // If provided with allowed servers, then filter out the projects that are not in the list
            let mut user_methods: HashMap<Ulid, showtimes_db::m::ServerUser> = servers
                .iter()
                .filter_map(|s| {
                    s.owners()
                        .iter()
                        .find(|&o| o.id == user_info.id())
                        .map(|o| (s.id(), o.clone()))
                })
                .collect();

            if let Some(creators) = &queries.creators {
                // Since creators is provided, remove the servers that are not in the list
                user_methods.retain(|k, _| creators.contains(k));
            }

            if user_methods.is_empty() {
                return GQLError::new(
                    "User does not have access to any of the allowed servers",
                    GQLErrorCode::UserInsufficientPrivilege,
                )
                .extend(|e| {
                    e.set("user_id", user_info.id().to_string());
                    e.set(
                        "servers",
                        servers
                            .iter()
                            .map(|s| s.id().to_string())
                            .collect::<Vec<String>>(),
                    );
                })
                .into();
            }

            let document_fetchs = user_methods
                .iter()
                .map(|(s, m)| {
                    if m.privilege == UserPrivilege::ProjectManager {
                        match &queries.ids {
                            Some(ids) => {
                                // remove the extras that is not in IDs
                                let req_ids: Vec<String> =
                                    ids.iter().map(|id| id.to_string()).collect();
                                let extras: Vec<String> = m
                                    .extras
                                    .iter()
                                    .filter(|id| req_ids.contains(id))
                                    .cloned()
                                    .collect();
                                doc! {
                                    "creator": s.to_string(),
                                    "id": { "$in": extras }
                                }
                            }
                            None => {
                                doc! {
                                    "creator": s.to_string(),
                                    "id": { "$in": m.extras.clone() }
                                }
                            }
                        }
                    } else {
                        match &queries.ids {
                            Some(ids) => {
                                let ids: Vec<String> =
                                    ids.iter().map(|id| id.to_string()).collect();
                                doc! {
                                    "creator": s.to_string(),
                                    "id": { "$in": ids }
                                }
                            }
                            None => {
                                doc! {
                                    "creator": s.to_string()
                                }
                            }
                        }
                    }
                })
                .collect::<Vec<Document>>();

            document_fetchs
        }
        _ => {
            // If not provided with allowed servers, then fetch all projects
            let all_queries = match (&queries.ids, &queries.creators) {
                (Some(ids), Some(creators)) => {
                    let ids: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
                    let creators: Vec<String> = creators.iter().map(|id| id.to_string()).collect();
                    doc! {
                        "$or": [
                            { "id": { "$in": ids } },
                            { "creator": { "$in": creators } }
                        ]
                    }
                }
                (Some(ids), None) => {
                    let ids: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
                    doc! {
                        "id": { "$in": ids }
                    }
                }
                (None, Some(creators)) => {
                    let creators: Vec<String> = creators.iter().map(|id| id.to_string()).collect();
                    doc! {
                        "creator": { "$in": creators }
                    }
                }
                (None, None) => Document::new(),
            };

            vec![all_queries]
        }
    };

    let (query_docs_fetch, query_count_fetch) = if fetch_docs.len() > 1 {
        match queries.cursor {
            Some(cursor) => (
                doc! {
                    "$or": fetch_docs.clone(),
                    "id": { "$gte": cursor.to_string() }
                },
                doc! {
                    "$or": fetch_docs,
                },
            ),
            None => (
                doc! {
                    "$or": fetch_docs.clone()
                },
                doc! {
                    "$or": fetch_docs
                },
            ),
        }
    } else {
        let count_query = fetch_docs.first().unwrap().clone();
        // Guaranteed to have at least one document
        let mut base_query = count_query.clone();
        match queries.cursor {
            Some(cursor) => {
                if queries.unpaged {
                    (base_query, count_query)
                } else {
                    let cursor = cursor.to_string();

                    // Extend $id query to include $gte
                    let entry = base_query.entry("id".to_string()).or_insert_with(|| {
                        showtimes_db::mongodb::bson::Bson::Document({
                            // empty doc
                            showtimes_db::mongodb::bson::Document::new()
                        })
                    });

                    if let showtimes_db::mongodb::bson::Bson::Document(doc) = entry {
                        doc.insert("$gte".to_string(), cursor);
                    }

                    (base_query, count_query)
                }
            }
            None => (base_query, count_query),
        }
    };

    let col = prj_handler.get_collection();
    let base_cursor = col
        .find(query_docs_fetch)
        .sort(queries.sort.into_sort_doc(Some("title".to_string())));

    let cursor = if queries.unpaged {
        base_cursor
    } else {
        base_cursor.limit((per_page + 1) as i64)
    }
    .await
    .extend_error(GQLErrorCode::ProjectRequestFails, |f_ctx| {
        queries.dump_query(f_ctx);
        f_ctx.set("where", GQLDataLoaderWhere::ProjectLoaderPaginated);
    })?;
    let count = prj_handler
        .get_collection()
        .count_documents(query_count_fetch)
        .await
        .extend_error(GQLErrorCode::ProjectRequestFails, |f_ctx| {
            queries.dump_query(f_ctx);
            f_ctx.set("where", GQLDataLoaderWhere::ProjectLoaderPaginatedCount);
        })?;

    let mut all_servers: Vec<showtimes_db::m::Project> =
        cursor
            .try_collect()
            .await
            .extend_error(GQLErrorCode::ProjectRequestFails, |f_ctx| {
                queries.dump_query(f_ctx);
                f_ctx.set("where", GQLDataLoaderWhere::ProjectLoaderCollect);
                f_ctx.set("where_req", GQLDataLoaderWhere::ProjectLoaderPaginatedCount);
            })?;

    if queries.unpaged {
        let page_info = PageInfoGQL::new(count, per_page, None);
        return Ok(PaginatedResult::new(all_servers, page_info));
    }

    // If all_servers is equal to per_page, then there is a next page
    let last_srv = if all_servers.len() > per_page as usize {
        Some(all_servers.pop().unwrap())
    } else {
        None
    };

    let page_info = PageInfoGQL::new(count, per_page, last_srv.map(|p| p.id.into()));

    Ok(PaginatedResult::new(all_servers, page_info))
}
