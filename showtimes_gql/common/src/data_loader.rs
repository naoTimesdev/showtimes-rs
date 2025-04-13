//! Data loader implementations

use std::ops::Deref;

use ahash::{HashMap, HashMapExt};
use async_graphql::{FieldError, dataloader::Loader};
use futures_util::TryStreamExt;
use showtimes_db::{
    DatabaseShared,
    mongodb::bson::{Document, doc},
};
use showtimes_shared::ulid::Ulid;

use crate::{GQLDataLoaderWhere, GQLErrorCode, GQLErrorExt};

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
                "id": { "$in": keys_to_string.clone() }
            })
            .limit(keys.len() as i64)
            .await
            .extend_error(GQLErrorCode::UserRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::UserLoaderId);
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::User>>()
            .await
            .extend_error(GQLErrorCode::UserRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::UserLoaderCollect);
                e.set("where_req", GQLDataLoaderWhere::UserLoaderId);
            })?;

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
                "discord_meta.id": { "$in": keys_to_string.clone() }
            })
            .limit(keys.len() as i64)
            .await
            .extend_error(GQLErrorCode::UserRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::UserLoaderDiscordId);
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::User>>()
            .await
            .extend_error(GQLErrorCode::UserRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::UserLoaderCollect);
                e.set("where_req", GQLDataLoaderWhere::UserLoaderDiscordId);
            })?;

        let mapped_res: HashMap<DiscordIdLoad, showtimes_db::m::User> = all_results
            .iter()
            .map(|u| (DiscordIdLoad(u.discord_meta.id.clone()), u.clone()))
            .collect();

        Ok(mapped_res)
    }
}

impl Loader<showtimes_shared::APIKey> for UserDataLoader {
    type Value = showtimes_db::m::User;
    type Error = FieldError;

    async fn load(
        &self,
        keys: &[showtimes_shared::APIKey],
    ) -> Result<HashMap<showtimes_shared::APIKey, Self::Value>, Self::Error> {
        let keys_to_string = keys.iter().map(|k| k.to_string()).collect::<Vec<_>>();
        let result = self
            .col
            .get_collection()
            .find(doc! {
                "api_key.key": { "$in": keys_to_string.clone() }
            })
            .limit(keys.len() as i64)
            .await
            .extend_error(GQLErrorCode::UserRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::UserLoaderAPIKey);
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::User>>()
            .await
            .extend_error(GQLErrorCode::UserRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::UserLoaderCollect);
                e.set("where_req", GQLDataLoaderWhere::UserLoaderAPIKey);
            })?;

        let mapped_res: HashMap<showtimes_shared::APIKey, showtimes_db::m::User> =
            all_results.iter().fold(HashMap::new(), |mut acc, item| {
                item.api_key.iter().for_each(|k| {
                    acc.entry(k.key).or_insert(item.clone());
                });
                acc
            });

        Ok(mapped_res)
    }
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

impl Loader<ServerOwnerId> for ProjectDataLoader {
    type Error = FieldError;
    type Value = Vec<showtimes_db::m::Project>;

    async fn load(
        &self,
        keys: &[ServerOwnerId],
    ) -> Result<HashMap<ServerOwnerId, Self::Value>, Self::Error> {
        let keys_to_string = keys.iter().map(|k| (*k).to_string()).collect::<Vec<_>>();
        let result = self
            .col
            .get_collection()
            .find(doc! {
                "creator": { "$in": keys_to_string.clone() }
            })
            .await
            .extend_error(GQLErrorCode::ProjectRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ProjectLoaderOwnerId);
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::Project>>()
            .await
            .extend_error(GQLErrorCode::ProjectRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ProjectLoaderCollect);
                e.set("where_req", GQLDataLoaderWhere::ProjectLoaderOwnerId);
            })?;

        let mapped_res: HashMap<ServerOwnerId, Vec<showtimes_db::m::Project>> =
            all_results.iter().fold(HashMap::new(), |mut acc, item| {
                acc.entry(ServerOwnerId::new(item.creator))
                    .or_default()
                    .push(item.clone());
                acc
            });

        Ok(mapped_res)
    }
}

impl Loader<Ulid> for ProjectDataLoader {
    type Error = FieldError;
    type Value = showtimes_db::m::Project;

    async fn load(&self, keys: &[Ulid]) -> Result<HashMap<Ulid, Self::Value>, Self::Error> {
        let keys_to_string = keys.iter().map(|k| k.to_string()).collect::<Vec<_>>();
        let result = self
            .col
            .get_collection()
            .find(doc! {
                "id": { "$in": keys_to_string.clone() }
            })
            .await
            .extend_error(GQLErrorCode::ProjectRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ProjectLoaderId);
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::Project>>()
            .await
            .extend_error(GQLErrorCode::ProjectRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ProjectLoaderCollect);
                e.set("where_req", GQLDataLoaderWhere::ProjectLoaderId);
            })?;

        let mapped_res: HashMap<Ulid, showtimes_db::m::Project> = all_results
            .iter()
            .map(|proj| (proj.id, proj.clone()))
            .collect();

        Ok(mapped_res)
    }
}

/// A simple owner data loader
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerOwnerId(Ulid);

impl ServerOwnerId {
    /// Initialize a new server owner ID
    pub fn new(id: Ulid) -> Self {
        Self(id)
    }
}

impl Deref for ServerOwnerId {
    type Target = Ulid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A simple server and owner ID data loader
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerAndOwnerId {
    server: Ulid,
    owner: Ulid,
}

impl ServerAndOwnerId {
    /// Initialize a new server and owner ID
    pub fn new(server: Ulid, owner: Ulid) -> Self {
        Self { server, owner }
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
                "id": { "$in": keys_to_string.clone() }
            })
            .limit(keys.len() as i64)
            .await
            .extend_error(GQLErrorCode::ServerRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ServerLoaderId);
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::Server>>()
            .await
            .extend_error(GQLErrorCode::ServerRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ServerLoaderCollect);
                e.set("where_req", GQLDataLoaderWhere::ServerLoaderId);
            })?;

        let mapped_res: HashMap<Ulid, showtimes_db::m::Server> =
            all_results.iter().map(|u| (u.id, u.clone())).collect();

        Ok(mapped_res)
    }
}

impl Loader<ServerOwnerId> for ServerDataLoader {
    type Value = Vec<showtimes_db::m::Server>;
    type Error = FieldError;

    async fn load(
        &self,
        keys: &[ServerOwnerId],
    ) -> Result<HashMap<ServerOwnerId, Self::Value>, Self::Error> {
        let keys_to_string = keys.iter().map(|k| (*k).to_string()).collect::<Vec<_>>();
        let result = self
            .col
            .get_collection()
            .find(doc! {
                "owners.id": { "$in": keys_to_string.clone() }
            })
            .limit(keys.len() as i64)
            .await
            .extend_error(GQLErrorCode::ServerRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ServerLoaderOwnerId);
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::Server>>()
            .await
            .extend_error(GQLErrorCode::ServerRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ServerLoaderCollect);
                e.set("where_req", GQLDataLoaderWhere::ServerLoaderOwnerId);
            })?;

        let mapped_res: HashMap<ServerOwnerId, Vec<showtimes_db::m::Server>> =
            all_results.iter().fold(HashMap::new(), |mut acc, item| {
                item.owners.iter().for_each(|o| {
                    acc.entry(ServerOwnerId::new(o.id))
                        .or_default()
                        .push(item.clone());
                });
                acc
            });

        Ok(mapped_res)
    }
}

impl Loader<ServerAndOwnerId> for ServerDataLoader {
    type Value = showtimes_db::m::Server;
    type Error = FieldError;

    async fn load(
        &self,
        keys: &[ServerAndOwnerId],
    ) -> Result<HashMap<ServerAndOwnerId, Self::Value>, Self::Error> {
        let all_keys_mappings: Vec<Document> = keys
            .iter()
            .map(|k| {
                doc! {
                    "id": k.server.to_string(),
                    "owners.id": k.owner.to_string()
                }
            })
            .collect();

        let result = self
            .col
            .get_collection()
            .find(doc! {
                "$or": all_keys_mappings
            })
            .limit(keys.len() as i64)
            .await
            .extend_error(GQLErrorCode::ServerRequestFails, |e| {
                e.set("where", GQLDataLoaderWhere::ServerLoaderIdOrOwnerId);
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::Server>>()
            .await
            .extend_error(GQLErrorCode::ServerRequestFails, |e| {
                e.set("where", GQLDataLoaderWhere::ServerLoaderCollect);
                e.set("where_req", GQLDataLoaderWhere::ServerLoaderIdOrOwnerId);
            })?;

        let mapped_res: HashMap<ServerAndOwnerId, showtimes_db::m::Server> =
            all_results.iter().fold(HashMap::new(), |mut acc, item| {
                item.owners.iter().for_each(|o| {
                    acc.entry(ServerAndOwnerId::new(item.id, o.id))
                        .or_insert(item.clone());
                });
                acc
            });

        Ok(mapped_res)
    }
}

/// A data loader for the server sync/collab model
pub struct ServerSyncLoader {
    col: showtimes_db::CollaborationSyncHandler,
}

impl ServerSyncLoader {
    /// Create a new server sync data loader
    pub fn new(col: &DatabaseShared) -> Self {
        let col = showtimes_db::CollaborationSyncHandler::new(col);
        ServerSyncLoader { col }
    }
}

/// A server sync server ID
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct ServerSyncServerId(Ulid);

impl ServerSyncServerId {
    /// Initialize a new server sync server ID
    pub fn new(id: Ulid) -> Self {
        Self(id)
    }
}

impl Deref for ServerSyncServerId {
    type Target = Ulid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A server sync project and server ID
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct ServerSyncIds {
    /// The server ID
    id: Ulid,
    /// Project ID
    project: Ulid,
}

impl ServerSyncIds {
    /// Initialize a new server sync project and server ID
    pub fn new(id: Ulid, project: Ulid) -> Self {
        Self { id, project }
    }
}

impl Loader<Ulid> for ServerSyncLoader {
    type Value = showtimes_db::m::ServerCollaborationSync;
    type Error = FieldError;

    async fn load(&self, keys: &[Ulid]) -> Result<HashMap<Ulid, Self::Value>, Self::Error> {
        let keys_to_string = keys.iter().map(|k| k.to_string()).collect::<Vec<_>>();

        let result = self
            .col
            .get_collection()
            .find(doc! {
                "id": { "$in": keys_to_string.clone() }
            })
            .limit(keys.len() as i64)
            .await
            .extend_error(GQLErrorCode::ServerSyncRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ServerSyncLoaderId);
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::ServerCollaborationSync>>()
            .await
            .extend_error(GQLErrorCode::ServerSyncRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ServerSyncLoaderCollect);
                e.set("where_req", GQLDataLoaderWhere::ServerSyncLoaderId);
            })?;

        let mapped_res: HashMap<Ulid, showtimes_db::m::ServerCollaborationSync> =
            all_results.iter().map(|u| (u.id, u.clone())).collect();

        Ok(mapped_res)
    }
}

impl Loader<ServerSyncServerId> for ServerSyncLoader {
    type Value = Vec<showtimes_db::m::ServerCollaborationSync>;
    type Error = FieldError;

    async fn load(
        &self,
        keys: &[ServerSyncServerId],
    ) -> Result<HashMap<ServerSyncServerId, Self::Value>, Self::Error> {
        let keys_to_string = keys.iter().map(|k| (*k).to_string()).collect::<Vec<_>>();

        let result = self
            .col
            .get_collection()
            .find(doc! {
                "projects.server": { "$in": keys_to_string.clone() }
            })
            .limit(keys.len() as i64)
            .await
            .extend_error(GQLErrorCode::ServerSyncRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ServerSyncLoaderServerId);
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::ServerCollaborationSync>>()
            .await
            .extend_error(GQLErrorCode::ServerSyncRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ServerSyncLoaderCollect);
                e.set("where_req", GQLDataLoaderWhere::ServerSyncLoaderServerId);
            })?;

        let mapped_res: HashMap<ServerSyncServerId, Vec<showtimes_db::m::ServerCollaborationSync>> =
            all_results.iter().fold(HashMap::new(), |mut acc, item| {
                item.projects.iter().for_each(|o| {
                    acc.entry(ServerSyncServerId::new(o.server))
                        .or_default()
                        .push(item.clone())
                });
                acc
            });

        Ok(mapped_res)
    }
}

impl Loader<ServerSyncIds> for ServerSyncLoader {
    type Error = FieldError;
    type Value = showtimes_db::m::ServerCollaborationSync;

    async fn load(
        &self,
        keys: &[ServerSyncIds],
    ) -> Result<HashMap<ServerSyncIds, Self::Value>, Self::Error> {
        let all_keys_mappings: Vec<Document> = keys
            .iter()
            .map(|k| {
                doc! {
                    "$and": [
                        {
                            "projects.server": k.id.to_string(),
                            "projects.project": k.project.to_string()
                        }
                    ]
                }
            })
            .collect();

        let result = self
            .col
            .get_collection()
            .find(doc! {
                "$or": all_keys_mappings
            })
            .limit(keys.len() as i64)
            .await
            .extend_error(GQLErrorCode::ServerSyncRequestFails, |e| {
                e.set(
                    "where",
                    GQLDataLoaderWhere::ServerSyncLoaderServerAndProjectId,
                );
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::ServerCollaborationSync>>()
            .await
            .extend_error(GQLErrorCode::ServerSyncRequestFails, |e| {
                e.set("where", GQLDataLoaderWhere::ServerSyncLoaderCollect);
                e.set(
                    "where_req",
                    GQLDataLoaderWhere::ServerSyncLoaderServerAndProjectId,
                );
            })?;

        let mapped_res: HashMap<ServerSyncIds, showtimes_db::m::ServerCollaborationSync> =
            all_results.iter().fold(HashMap::new(), |mut acc, item| {
                item.projects.iter().for_each(|o| {
                    acc.entry(ServerSyncIds::new(o.server, o.project))
                        .or_insert(item.clone());
                });
                acc
            });

        Ok(mapped_res)
    }
}

/// A data loader for the server collab invite model
pub struct ServerInviteLoader {
    col: showtimes_db::CollaborationInviteHandler,
}

impl ServerInviteLoader {
    /// Create a new server sync data loader
    pub fn new(col: &DatabaseShared) -> Self {
        let col = showtimes_db::CollaborationInviteHandler::new(col);
        ServerInviteLoader { col }
    }
}

impl Loader<Ulid> for ServerInviteLoader {
    type Value = showtimes_db::m::ServerCollaborationInvite;
    type Error = FieldError;

    async fn load(&self, keys: &[Ulid]) -> Result<HashMap<Ulid, Self::Value>, Self::Error> {
        let keys_to_string = keys.iter().map(|k| k.to_string()).collect::<Vec<_>>();
        let result = self
            .col
            .get_collection()
            .find(doc! {
                "id": { "$in": keys_to_string.clone() }
            })
            .await
            .extend_error(GQLErrorCode::ServerInviteRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ServerSyncLoaderId);
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::ServerCollaborationInvite>>()
            .await
            .extend_error(GQLErrorCode::ServerInviteRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ServerSyncInviteLoaderCollect);
                e.set("where_req", GQLDataLoaderWhere::ServerSyncLoaderId);
            })?;

        let mapped_res: HashMap<Ulid, showtimes_db::m::ServerCollaborationInvite> = all_results
            .iter()
            .map(|proj| (proj.id, proj.clone()))
            .collect();

        Ok(mapped_res)
    }
}

/// A data loader for the RSS feed model
pub struct RSSFeedLoader {
    col: showtimes_db::RSSFeedHandler,
}

impl RSSFeedLoader {
    /// Create a new server sync data loader
    pub fn new(col: &DatabaseShared) -> Self {
        let col = showtimes_db::RSSFeedHandler::new(col);
        RSSFeedLoader { col }
    }

    /// Get the collection handler
    pub fn get_inner(&self) -> &showtimes_db::RSSFeedHandler {
        &self.col
    }
}

impl Loader<Ulid> for RSSFeedLoader {
    type Value = showtimes_db::m::RSSFeed;
    type Error = FieldError;

    async fn load(&self, keys: &[Ulid]) -> Result<HashMap<Ulid, Self::Value>, Self::Error> {
        let keys_to_string = keys.iter().map(|k| k.to_string()).collect::<Vec<_>>();
        let result = self
            .col
            .get_collection()
            .find(doc! {
                "id": { "$in": keys_to_string.clone() }
            })
            .await
            .extend_error(GQLErrorCode::RSSFeedRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::RSSFeedLoaderId);
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::RSSFeed>>()
            .await
            .extend_error(GQLErrorCode::RSSFeedRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::RSSFeedLoaderCollect);
                e.set("where_req", GQLDataLoaderWhere::RSSFeedLoaderId);
            })?;

        let mapped_res: HashMap<Ulid, showtimes_db::m::RSSFeed> = all_results
            .iter()
            .map(|rss| (rss.id, rss.clone()))
            .collect();

        Ok(mapped_res)
    }
}

/// A data loader key for the RSS feed
///
/// Based on the server ULID
#[derive(Clone)]
pub struct RSSFeedServer(Ulid);

impl std::hash::Hash for RSSFeedServer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl std::cmp::PartialEq for RSSFeedServer {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl std::cmp::Eq for RSSFeedServer {}

impl Deref for RSSFeedServer {
    type Target = Ulid;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Ulid> for RSSFeedServer {
    fn from(id: Ulid) -> Self {
        RSSFeedServer(id)
    }
}

impl Loader<RSSFeedServer> for RSSFeedLoader {
    type Value = Vec<showtimes_db::m::RSSFeed>;
    type Error = FieldError;

    async fn load(
        &self,
        keys: &[RSSFeedServer],
    ) -> Result<HashMap<RSSFeedServer, Self::Value>, Self::Error> {
        let keys_to_string = keys.iter().map(|k| k.0.to_string()).collect::<Vec<_>>();
        let result = self
            .col
            .get_collection()
            .find(doc! {
                "creator": { "$in": keys_to_string.clone() }
            })
            .await
            .extend_error(GQLErrorCode::RSSFeedRequestFails, |e| {
                e.set("creator", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::RSSFeedLoaderId);
            })?;

        let all_results: Vec<showtimes_db::m::RSSFeed> = result
            .try_collect::<Vec<showtimes_db::m::RSSFeed>>()
            .await
            .extend_error(GQLErrorCode::RSSFeedRequestFails, |e| {
                e.set("creator", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::RSSFeedLoaderCollect);
                e.set("where_req", GQLDataLoaderWhere::RSSFeedLoaderId);
            })?;

        let mapped_res: HashMap<RSSFeedServer, Vec<showtimes_db::m::RSSFeed>> =
            all_results.iter().fold(HashMap::new(), |mut acc, item| {
                acc.entry(RSSFeedServer::from(item.creator))
                    .or_default()
                    .push(item.clone());
                acc
            });

        Ok(mapped_res)
    }
}

/// A data loader for the RSS feed model
pub struct ServerPremiumLoader {
    col: showtimes_db::ServerPremiumHandler,
}

impl ServerPremiumLoader {
    /// Create a new server sync data loader
    pub fn new(col: &DatabaseShared) -> Self {
        let col = showtimes_db::ServerPremiumHandler::new(col);
        ServerPremiumLoader { col }
    }

    /// Get the collection handler
    pub fn get_inner(&self) -> &showtimes_db::ServerPremiumHandler {
        &self.col
    }
}

impl Loader<Ulid> for ServerPremiumLoader {
    type Value = showtimes_db::m::ServerPremium;
    type Error = FieldError;

    async fn load(&self, keys: &[Ulid]) -> Result<HashMap<Ulid, Self::Value>, Self::Error> {
        let keys_to_string = keys.iter().map(|k| k.to_string()).collect::<Vec<_>>();
        let result = self
            .col
            .get_collection()
            .find(doc! {
                "id": { "$in": keys_to_string.clone() }
            })
            .await
            .extend_error(GQLErrorCode::ServerPremiumRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ServerPremiumLoaderId);
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::ServerPremium>>()
            .await
            .extend_error(GQLErrorCode::ServerPremiumRequestFails, |e| {
                e.set("ids", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ServerPremiumLoaderCollect);
                e.set("where_req", GQLDataLoaderWhere::ServerPremiumLoaderId);
            })?;

        let mapped_res: HashMap<Ulid, showtimes_db::m::ServerPremium> = all_results
            .iter()
            .map(|rss| (rss.id, rss.clone()))
            .collect();

        Ok(mapped_res)
    }
}

/// A data loader server key for the Server Premium
///
/// Based on the server ULID
#[derive(Clone)]
pub struct ServerPremiumServer(Ulid);

impl std::hash::Hash for ServerPremiumServer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl std::cmp::PartialEq for ServerPremiumServer {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl std::cmp::Eq for ServerPremiumServer {}

impl Deref for ServerPremiumServer {
    type Target = Ulid;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Ulid> for ServerPremiumServer {
    fn from(id: Ulid) -> Self {
        ServerPremiumServer(id)
    }
}

impl Loader<ServerPremiumServer> for ServerPremiumLoader {
    type Value = Vec<showtimes_db::m::ServerPremium>;
    type Error = FieldError;

    async fn load(
        &self,
        keys: &[ServerPremiumServer],
    ) -> Result<HashMap<ServerPremiumServer, Self::Value>, Self::Error> {
        let keys_to_string = keys.iter().map(|k| k.0.to_string()).collect::<Vec<_>>();
        let result = self
            .col
            .get_collection()
            .find(doc! {
                "target": { "$in": keys_to_string.clone() }
            })
            .await
            .extend_error(GQLErrorCode::ServerPremiumRequestFails, |e| {
                e.set("creator", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ServerPremiumLoaderServerId);
            })?;

        let all_results: Vec<showtimes_db::m::ServerPremium> = result
            .try_collect::<Vec<showtimes_db::m::ServerPremium>>()
            .await
            .extend_error(GQLErrorCode::ServerPremiumRequestFails, |e| {
                e.set("creator", keys_to_string.clone());
                e.set("where", GQLDataLoaderWhere::ServerPremiumLoaderCollect);
                e.set("where_req", GQLDataLoaderWhere::ServerPremiumLoaderServerId);
            })?;

        let mapped_res: HashMap<ServerPremiumServer, Vec<showtimes_db::m::ServerPremium>> =
            all_results.iter().fold(HashMap::new(), |mut acc, item| {
                acc.entry(ServerPremiumServer::from(item.target))
                    .or_default()
                    .push(item.clone());
                acc
            });

        Ok(mapped_res)
    }
}
