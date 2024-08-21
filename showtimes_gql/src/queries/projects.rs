use futures::TryStreamExt;
use showtimes_db::{
    mongodb::bson::{doc, Document},
    DatabaseShared,
};
use showtimes_shared::ulid::Ulid;

use crate::models::{
    prelude::{PageInfoGQL, PaginatedGQL},
    projects::ProjectGQL,
};

/// The config for querying servers
#[derive(Debug, Clone, Default)]
pub struct ProjectQuery {
    /// Specify project IDs to query
    ids: Option<Vec<Ulid>>,
    /// The number of servers to return
    per_page: Option<u32>,
    /// The cursor to start from
    cursor: Option<Ulid>,
    /// The server that created the project
    creators: Option<Vec<Ulid>>,
}

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
    pub fn with_creators(mut self, user: Vec<Ulid>) -> Self {
        self.creators = Some(user);
        self
    }

    pub fn set_creators(&mut self, user: &[Ulid]) {
        self.creators = Some(user.to_vec());
    }
}

pub async fn query_projects_paginated(
    ctx: &async_graphql::Context<'_>,
    queries: ProjectQuery,
) -> async_graphql::Result<PaginatedGQL<ProjectGQL>> {
    let db = ctx.data_unchecked::<DatabaseShared>();

    // Allowed range of per_page is 10-100, with
    let per_page = queries
        .per_page
        .filter(|&p| (2..=100).contains(&p))
        .unwrap_or(20);

    let prj_handler = showtimes_db::ProjectHandler::new(db);

    let mut base_query = match (queries.ids, queries.creators) {
        (Some(ids), Some(creators)) => {
            let ids: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();
            let creators: Vec<String> = creators.into_iter().map(|id| id.to_string()).collect();
            doc! {
                "$or": [
                    { "id": { "$in": ids } },
                    { "creator": { "$in": creators } }
                ]
            }
        }
        (Some(ids), None) => {
            let ids: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();
            doc! {
                "id": { "$in": ids }
            }
        }
        (None, Some(creators)) => {
            let creators: Vec<String> = creators.into_iter().map(|id| id.to_string()).collect();
            doc! {
                "creator": { "$in": creators }
            }
        }
        (None, None) => Document::new(),
    };

    let count_query = base_query.clone();
    let doc_query = if let Some(cursor) = queries.cursor {
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

        base_query
    } else {
        base_query
    };

    let cursor = prj_handler
        .get_collection()
        .find(doc_query)
        .limit((per_page + 1) as i64)
        .sort(doc! { "id": 1 })
        .await?;
    let count = prj_handler
        .get_collection()
        .count_documents(count_query)
        .await?;

    let mut all_servers: Vec<showtimes_db::m::Project> = cursor.try_collect().await?;

    // If all_servers is equal to per_page, then there is a next page
    let last_srv = if all_servers.len() > per_page as usize {
        Some(all_servers.pop().unwrap())
    } else {
        None
    };

    let page_info = PageInfoGQL::new(count, per_page, last_srv.map(|p| p.id.into()));

    Ok(PaginatedGQL::new(
        all_servers.into_iter().map(|p| p.into()).collect(),
        page_info,
    ))
}
