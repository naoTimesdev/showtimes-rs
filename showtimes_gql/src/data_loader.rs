use std::{collections::HashMap, sync::OnceLock};

use async_graphql::{
    dataloader::{DataLoader, Loader},
    Context, FieldError,
};
use futures::TryStreamExt;
use showtimes_db::{mongodb::bson::doc, DatabaseShared};
use showtimes_session::ShowtimesUserSession;
use showtimes_shared::ulid::Ulid;

static STUBBED_OWNER: OnceLock<showtimes_db::m::User> = OnceLock::new();

/// A simple data loader for Discord IDs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiscordIdLoad(pub String);

/// A simple data loader for API keys
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApiKeyLoad(pub String);

/// A data loader for the user model
pub struct UserDataLoader {
    col: showtimes_db::UserHandler,
}

impl UserDataLoader {
    /// Create a new user data loader
    pub fn new(col: &DatabaseShared) -> Self {
        let col = showtimes_db::UserHandler::new(col);
        UserDataLoader { col }
    }
}

impl Loader<Ulid> for UserDataLoader {
    type Value = showtimes_db::m::User;
    type Error = FieldError;

    async fn load(&self, keys: &[Ulid]) -> Result<HashMap<Ulid, Self::Value>, Self::Error> {
        let keys_to_string = keys.iter().map(|k| k.to_string()).collect::<Vec<_>>();
        let result = self
            .col
            .get_collection()
            .find(doc! {
                "id": { "$in": keys_to_string }
            })
            .limit(keys.len() as i64)
            .await?;

        let all_results = result.try_collect::<Vec<showtimes_db::m::User>>().await?;
        let mapped_res: HashMap<Ulid, showtimes_db::m::User> =
            all_results.iter().map(|u| (u.id, u.clone())).collect();

        Ok(mapped_res)
    }
}

impl Loader<DiscordIdLoad> for UserDataLoader {
    type Value = showtimes_db::m::User;
    type Error = FieldError;

    async fn load(
        &self,
        keys: &[DiscordIdLoad],
    ) -> Result<HashMap<DiscordIdLoad, Self::Value>, Self::Error> {
        let keys_to_string = keys.iter().map(|k| k.0.clone()).collect::<Vec<_>>();
        let result = self
            .col
            .get_collection()
            .find(doc! {
                "discord_meta.id": { "$in": keys_to_string }
            })
            .limit(keys.len() as i64)
            .await?;

        let all_results = result.try_collect::<Vec<showtimes_db::m::User>>().await?;
        let mapped_res: HashMap<DiscordIdLoad, showtimes_db::m::User> = all_results
            .iter()
            .map(|u| (DiscordIdLoad(u.discord_meta.id.clone()), u.clone()))
            .collect();

        Ok(mapped_res)
    }
}

impl Loader<ApiKeyLoad> for UserDataLoader {
    type Value = showtimes_db::m::User;
    type Error = FieldError;

    async fn load(
        &self,
        keys: &[ApiKeyLoad],
    ) -> Result<HashMap<ApiKeyLoad, Self::Value>, Self::Error> {
        let keys_to_string = keys.iter().map(|k| k.0.clone()).collect::<Vec<_>>();
        let result = self
            .col
            .get_collection()
            .find(doc! {
                "api_key": { "$in": keys_to_string }
            })
            .limit(keys.len() as i64)
            .await?;

        let all_results = result.try_collect::<Vec<showtimes_db::m::User>>().await?;
        let mapped_res: HashMap<ApiKeyLoad, showtimes_db::m::User> = all_results
            .iter()
            .map(|u| (ApiKeyLoad(u.api_key.clone()), u.clone()))
            .collect();

        Ok(mapped_res)
    }
}

/// A data loader key to load project model
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ProjectDataLoaderKey {
    /// Load by ULID
    Id(Ulid),
    /// Load by server ID
    Server(Ulid),
}

/// A data loader for the project model
pub struct ProjectDataLoader {
    col: showtimes_db::ProjectHandler,
}

impl ProjectDataLoader {
    /// Create a new user data loader
    pub fn new(col: &DatabaseShared) -> Self {
        let col = showtimes_db::ProjectHandler::new(col);
        ProjectDataLoader { col }
    }
}

impl Loader<ProjectDataLoaderKey> for ProjectDataLoader {
    type Value = showtimes_db::m::Project;
    type Error = FieldError;

    async fn load(
        &self,
        keys: &[ProjectDataLoaderKey],
    ) -> Result<HashMap<ProjectDataLoaderKey, Self::Value>, Self::Error> {
        let fetch_by_ids: Vec<String> = keys
            .iter()
            .filter_map(|k| match k {
                ProjectDataLoaderKey::Id(id) => Some(id.to_string()),
                _ => None,
            })
            .collect();
        let fetch_by_servers: Vec<String> = keys
            .iter()
            .filter_map(|k| match k {
                ProjectDataLoaderKey::Server(id) => Some(id.to_string()),
                _ => None,
            })
            .collect();

        // tokio task
        let col_ids = self.col.get_collection();
        let col_creator = self.col.get_collection();
        let doc_fetch_ids = doc! {
            "api_key": { "$in": fetch_by_ids.clone() }
        };
        let mut tasks = vec![];
        tasks.push(tokio::spawn(async move {
            match col_ids
                .find(doc_fetch_ids)
                .limit(fetch_by_ids.len() as i64)
                .await
            {
                Ok(cursor) => {
                    let results = cursor.try_collect::<Vec<showtimes_db::m::Project>>().await;
                    results
                }
                Err(e) => Err(e),
            }
        }));
        let doc_fetch_creator = doc! {
            "creator": { "$in": fetch_by_servers.clone() }
        };
        tasks.push(tokio::spawn(async move {
            match col_creator
                .find(doc_fetch_creator)
                .limit(fetch_by_servers.len() as i64)
                .await
            {
                Ok(cursor) => {
                    let results = cursor.try_collect::<Vec<showtimes_db::m::Project>>().await;
                    results
                }
                Err(e) => Err(e),
            }
        }));

        let tasks_fut = futures::future::join_all(tasks).await;
        // Guaranteed to have 2 tasks
        let ids_task = tasks_fut.get(0).unwrap().as_ref()?.as_ref()?;
        let creator_task = tasks_fut.get(1).unwrap().as_ref()?.as_ref()?;

        let mapped_ids = ids_task
            .iter()
            .map(|u| (ProjectDataLoaderKey::Id(u.id), u.clone()))
            .collect::<HashMap<_, _>>();
        let mapped_creator = creator_task
            .iter()
            .map(|u| (ProjectDataLoaderKey::Server(u.creator), u.clone()))
            .collect::<HashMap<_, _>>();

        let mut mapped_res = HashMap::new();
        mapped_res.extend(mapped_ids);
        mapped_res.extend(mapped_creator);

        Ok(mapped_res)
    }
}

/// A data loader for the server model
pub struct ServerDataLoader {
    col: showtimes_db::ServerHandler,
}

impl ServerDataLoader {
    /// Create a new user data loader
    pub fn new(col: &DatabaseShared) -> Self {
        let col = showtimes_db::ServerHandler::new(col);
        ServerDataLoader { col }
    }
}

impl Loader<Ulid> for ServerDataLoader {
    type Value = showtimes_db::m::Server;
    type Error = FieldError;

    async fn load(&self, keys: &[Ulid]) -> Result<HashMap<Ulid, Self::Value>, Self::Error> {
        let keys_to_string = keys.iter().map(|k| k.to_string()).collect::<Vec<_>>();
        let result = self
            .col
            .get_collection()
            .find(doc! {
                "id": { "$in": keys_to_string }
            })
            .limit(keys.len() as i64)
            .await?;

        let all_results = result.try_collect::<Vec<showtimes_db::m::Server>>().await?;
        let mapped_res: HashMap<Ulid, showtimes_db::m::Server> =
            all_results.iter().map(|u| (u.id, u.clone())).collect();

        Ok(mapped_res)
    }
}

pub(crate) async fn find_authenticated_user(
    ctx: &Context<'_>,
) -> async_graphql::Result<showtimes_db::m::User> {
    let session = ctx.data_unchecked::<ShowtimesUserSession>();
    let loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();

    let load_method = match session.get_claims().get_audience() {
        showtimes_session::ShowtimesAudience::User => {
            // load as ULID
            let user_id =
                showtimes_shared::ulid::Ulid::from_string(session.get_claims().get_metadata())?;

            loader.load_one(user_id).await
        }
        showtimes_session::ShowtimesAudience::APIKey => {
            // load as API key
            let api_key = session.get_claims().get_metadata();

            loader.load_one(ApiKeyLoad(api_key.to_string())).await
        }
        showtimes_session::ShowtimesAudience::MasterKey => {
            let result = STUBBED_OWNER.get_or_init(|| {
                showtimes_db::m::User::stub_owner(session.get_claims().get_metadata())
            });

            Ok(Some(result.clone()))
        }
        _ => {
            return Err(FieldError::new("Invalid audience type for this session"));
        }
    };

    match load_method {
        Ok(Some(user)) => Ok(user),
        Ok(None) => Err(FieldError::new("User not found")),
        Err(e) => Err(e),
    }
}
