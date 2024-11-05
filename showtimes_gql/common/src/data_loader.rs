//! Data loader implementations

use std::{collections::HashMap, ops::Deref, sync::OnceLock};

use async_graphql::{
    dataloader::{DataLoader, Loader},
    Context, ErrorExtensions, FieldError,
};
use futures_util::TryStreamExt;
use showtimes_db::{
    mongodb::bson::{doc, Document},
    DatabaseShared,
};
use showtimes_session::ShowtimesUserSession;
use showtimes_shared::ulid::Ulid;

use crate::{GQLDataLoaderWhere, GQLError};

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
                "id": { "$in": keys_to_string.clone() }
            })
            .limit(keys.len() as i64)
            .await
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::UserLoaderId);
                    e.set("reason", GQLError::UserRequestFails);
                    e.set("code", GQLError::UserRequestFails.code());
                })
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::User>>()
            .await
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::UserLoaderCollect);
                    e.set("where_req", GQLDataLoaderWhere::UserLoaderId);
                    e.set("reason", GQLError::UserRequestFails);
                    e.set("code", GQLError::UserRequestFails.code());
                })
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
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::UserLoaderDiscordId);
                    e.set("reason", GQLError::UserRequestFails);
                    e.set("code", GQLError::UserRequestFails.code());
                })
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::User>>()
            .await
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::UserLoaderCollect);
                    e.set("where_req", GQLDataLoaderWhere::UserLoaderDiscordId);
                    e.set("reason", GQLError::UserRequestFails);
                    e.set("code", GQLError::UserRequestFails.code());
                })
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
                "api_key": { "$in": keys_to_string.clone() }
            })
            .limit(keys.len() as i64)
            .await
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::UserLoaderAPIKey);
                    e.set("reason", GQLError::UserRequestFails);
                    e.set("code", GQLError::UserRequestFails.code());
                })
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::User>>()
            .await
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::UserLoaderCollect);
                    e.set("where_req", GQLDataLoaderWhere::UserLoaderAPIKey);
                    e.set("reason", GQLError::UserRequestFails);
                    e.set("code", GQLError::UserRequestFails.code());
                })
            })?;
        let mapped_res: HashMap<showtimes_shared::APIKey, showtimes_db::m::User> =
            all_results.iter().map(|u| (u.api_key, u.clone())).collect();

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
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::ProjectLoaderOwnerId);
                    e.set("reason", GQLError::ProjectRequestFails);
                    e.set("code", GQLError::ProjectRequestFails.code());
                })
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::Project>>()
            .await
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::ProjectLoaderCollect);
                    e.set("where_req", GQLDataLoaderWhere::ProjectLoaderOwnerId);
                    e.set("reason", GQLError::ProjectRequestFails);
                    e.set("code", GQLError::ProjectRequestFails.code());
                })
            })?;
        let mapped_res: HashMap<ServerOwnerId, Vec<showtimes_db::m::Project>> = keys
            .iter()
            .map(|k| {
                let res = all_results
                    .iter()
                    .filter(|u| u.creator == **k)
                    .cloned()
                    .collect();

                (k.clone(), res)
            })
            .collect();

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
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::ProjectLoaderId);
                    e.set("reason", GQLError::ProjectRequestFails);
                    e.set("code", GQLError::ProjectRequestFails.code());
                })
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::Project>>()
            .await
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::ProjectLoaderCollect);
                    e.set("where_req", GQLDataLoaderWhere::ProjectLoaderId);
                    e.set("reason", GQLError::ProjectRequestFails);
                    e.set("code", GQLError::ProjectRequestFails.code());
                })
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
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::ServerLoaderId);
                    e.set("reason", GQLError::ServerRequestFails);
                    e.set("code", GQLError::ServerRequestFails.code());
                })
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::Server>>()
            .await
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::ServerLoaderCollect);
                    e.set("where_req", GQLDataLoaderWhere::ServerLoaderId);
                    e.set("reason", GQLError::ServerRequestFails);
                    e.set("code", GQLError::ServerRequestFails.code());
                })
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
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::ServerLoaderOwnerId);
                    e.set("reason", GQLError::ServerRequestFails);
                    e.set("code", GQLError::ServerRequestFails.code());
                })
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::Server>>()
            .await
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::ServerLoaderCollect);
                    e.set("where_req", GQLDataLoaderWhere::ServerLoaderOwnerId);
                    e.set("reason", GQLError::ServerRequestFails);
                    e.set("code", GQLError::ServerRequestFails.code());
                })
            })?;
        let mapped_res: HashMap<ServerOwnerId, Vec<showtimes_db::m::Server>> = keys
            .iter()
            .map(|k| {
                let res = all_results
                    .iter()
                    .filter(|u| u.owners.iter().any(|o| o.id == **k))
                    .cloned()
                    .collect();

                (k.clone(), res)
            })
            .collect();

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
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("where", GQLDataLoaderWhere::ServerLoaderIdOrOwnerId);
                    e.set("reason", GQLError::ServerRequestFails);
                    e.set("code", GQLError::ServerRequestFails.code());
                })
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::Server>>()
            .await
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("where", GQLDataLoaderWhere::ServerLoaderCollect);
                    e.set("where_req", GQLDataLoaderWhere::ServerLoaderIdOrOwnerId);
                    e.set("reason", GQLError::ServerRequestFails);
                    e.set("code", GQLError::ServerRequestFails.code());
                })
            })?;
        let mapped_res: HashMap<ServerAndOwnerId, showtimes_db::m::Server> = keys
            .iter()
            .filter_map(|k| {
                let res = all_results
                    .iter()
                    .find(|u| u.id == k.server && u.owners.iter().any(|o| o.id == k.owner));

                res.map(|r| (k.clone(), r.clone()))
            })
            .collect();

        Ok(mapped_res)
    }
}

/// A data loader for the server sync model
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
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::ServerSyncLoaderId);
                    e.set("reason", GQLError::ServerSyncRequestFails);
                    e.set("code", GQLError::ServerSyncRequestFails.code());
                })
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::ServerCollaborationSync>>()
            .await
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::ServerSyncLoaderCollect);
                    e.set("where_req", GQLDataLoaderWhere::ServerSyncLoaderId);
                    e.set("reason", GQLError::ServerSyncRequestFails);
                    e.set("code", GQLError::ServerSyncRequestFails.code());
                })
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
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::ServerSyncLoaderServerId);
                    e.set("reason", GQLError::ServerSyncRequestFails);
                    e.set("code", GQLError::ServerSyncRequestFails.code());
                })
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::ServerCollaborationSync>>()
            .await
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("ids", keys_to_string.clone());
                    e.set("where", GQLDataLoaderWhere::ServerSyncLoaderCollect);
                    e.set("where_req", GQLDataLoaderWhere::ServerSyncLoaderServerId);
                    e.set("reason", GQLError::ServerSyncRequestFails);
                    e.set("code", GQLError::ServerSyncRequestFails.code());
                })
            })?;
        let mapped_res: HashMap<ServerSyncServerId, Vec<showtimes_db::m::ServerCollaborationSync>> =
            keys.iter()
                .map(|k| {
                    let res: Vec<showtimes_db::m::ServerCollaborationSync> = all_results
                        .iter()
                        .filter(|u| u.projects.iter().any(|p| p.server == **k))
                        .cloned()
                        .collect();

                    (k.clone(), res)
                })
                .collect();

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
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set(
                        "where",
                        GQLDataLoaderWhere::ServerSyncLoaderServerAndProjectId,
                    );
                    e.set("reason", GQLError::ServerSyncRequestFails);
                    e.set("code", GQLError::ServerSyncRequestFails.code());
                })
            })?;

        let all_results = result
            .try_collect::<Vec<showtimes_db::m::ServerCollaborationSync>>()
            .await
            .map_err(|err| {
                err.extend_with(|_, e| {
                    e.set("where", GQLDataLoaderWhere::ServerSyncLoaderCollect);
                    e.set(
                        "where_req",
                        GQLDataLoaderWhere::ServerSyncLoaderServerAndProjectId,
                    );
                    e.set("reason", GQLError::ServerSyncRequestFails);
                    e.set("code", GQLError::ServerSyncRequestFails.code());
                })
            })?;
        let mapped_res: HashMap<ServerSyncIds, showtimes_db::m::ServerCollaborationSync> = keys
            .iter()
            .filter_map(|k| {
                let res = all_results.iter().find(|u| {
                    u.projects
                        .iter()
                        .any(|p| p.server == k.id && p.project == k.project)
                });

                res.map(|r| (k.clone(), r.clone()))
            })
            .collect();

        Ok(mapped_res)
    }
}

/// Find the current authenticated user
///
/// Returns an error when fails to load or find the user
pub async fn find_authenticated_user(
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
            let parse_api = showtimes_shared::APIKey::try_from(api_key)?;

            loader.load_one(parse_api).await
        }
        showtimes_session::ShowtimesAudience::MasterKey => {
            let result = STUBBED_OWNER.get_or_init(|| {
                showtimes_db::m::User::stub_owner(session.get_claims().get_metadata())
            });

            Ok(Some(result.clone()))
        }
        _ => {
            return Err(
                async_graphql::Error::new("Invalid audience type for this session").extend_with(
                    |_, e| {
                        e.set("metadata", session.get_claims().get_metadata());
                        e.set("reason", GQLError::UserInvalidAudience);
                        e.set("code", GQLError::UserInvalidAudience.code());
                    },
                ),
            );
        }
    };

    match load_method {
        Ok(Some(user)) => Ok(user),
        Ok(None) => Err(
            async_graphql::Error::new("User not found").extend_with(|_, e| {
                e.set("reason", GQLError::UserUnauthorized);
                e.set("code", GQLError::UserUnauthorized.code());
            }),
        ),
        Err(e) => Err(e),
    }
}
