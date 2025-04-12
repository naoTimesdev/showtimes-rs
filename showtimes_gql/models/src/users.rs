//! A users models list

use async_graphql::{Object, SimpleObject};
use errors::GQLError;
use showtimes_db::m::APIKeyCapability;
use showtimes_db::mongodb::bson::doc;
use showtimes_gql_common::{queries::ServerQueryUser, *};
use showtimes_gql_paginator::servers::ServerQuery;

use crate::common::PaginatedGQL;

use super::servers::ServerGQL;

/// The main user object
pub struct UserGQL {
    id: showtimes_shared::ulid::Ulid,
    username: String,
    kind: showtimes_db::m::UserKind,
    api_key: Vec<showtimes_db::m::APIKey>,
    registered: bool,
    avatar: Option<showtimes_db::m::ImageMetadata>,
    created: jiff::Timestamp,
    updated: jiff::Timestamp,
    disallow_server_fetch: bool,
    requester: Option<ServerQueryUser>,
}

/// The API key and the capabilities associated with it
#[derive(SimpleObject)]
#[graphql(name = "APIKeyDataGQL")]
pub struct APIKeyDataGQL {
    /// The API key
    key: APIKeyGQL,
    /// The capabilities associated with the API key
    capabilities: Vec<APIKeyCapabilityGQL>,
}

#[Object]
impl UserGQL {
    /// The user's ID
    async fn id(&self) -> UlidGQL {
        self.id.into()
    }

    /// The user's username
    async fn username(&self) -> String {
        self.username.clone()
    }

    /// The user's kind
    async fn kind(&self) -> UserKindGQL {
        self.kind.into()
    }

    /// The user's API key, this will be `null` if you're not *this* user.
    async fn api_key(&self) -> Option<Vec<APIKeyDataGQL>> {
        if let Some(requester) = self.requester {
            // Only return the API key if the requester is the same user or the requester is not a user
            if requester.id() == self.id || requester.kind() != showtimes_db::m::UserKind::User {
                Some(self.api_key.iter().map(APIKeyDataGQL::from).collect())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Check if the user is registered
    async fn registered(&self) -> bool {
        self.registered
    }

    /// The user's avatar
    async fn avatar(&self) -> Option<ImageMetadataGQL> {
        self.avatar.clone().map(|a| a.into())
    }

    /// The user's creation date
    async fn created(&self) -> DateTimeGQL {
        self.created.into()
    }

    /// The user's last update date
    async fn updated(&self) -> DateTimeGQL {
        self.updated.into()
    }

    /// Get the server associated with the user
    #[graphql(
        guard = "guard::AuthAPIKeyMinimumGuard::new(guard::APIKeyVerify::Specific(APIKeyCapability::QueryServers))"
    )]
    async fn servers(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Specify server IDs to query")] ids: Option<Vec<UlidGQL>>,
        #[graphql(
            name = "perPage",
            desc = "The number of servers to return, default to 20",
            validator(minimum = 2, maximum = 100)
        )]
        per_page: Option<u32>,
        #[graphql(desc = "The cursor to start from")] cursor: Option<UlidGQL>,
        #[graphql(desc = "Sort order, default to ID_ASC")] sort: Option<SortOrderGQL>,
    ) -> async_graphql::Result<PaginatedGQL<ServerGQL>> {
        if self.disallow_server_fetch {
            return GQLError::new(
                "Servers fetch from this context is disabled to avoid looping",
                GQLErrorCode::ServerFetchDisabled,
            )
            .extend(|e| {
                e.set("id", self.id.to_string());
                e.set("root", "user");
            })
            .into();
        }

        let mut queries =
            ServerQuery::new().with_current_user(ServerQueryUser::new(self.id, self.kind));
        if let Some(ids) = ids {
            queries.set_ids(ids.into_iter().map(|id| *id).collect());
        };
        if let Some(per_page) = per_page {
            queries.set_per_page(per_page);
        }
        if let Some(cursor) = cursor {
            queries.set_cursor(*cursor);
        }
        if let Some(sort) = sort {
            queries.set_sort(sort);
        }

        let results =
            showtimes_gql_paginator::servers::query_servers_paginated(ctx, queries).await?;

        let mapped_nodes: Vec<ServerGQL> = results
            .nodes()
            .iter()
            .map(|s| {
                let s_gql = ServerGQL::from(s);

                s_gql.with_current_user(self.id)
            })
            .collect();

        Ok(PaginatedGQL::new(mapped_nodes, *results.page_info()))
    }
}

impl From<showtimes_db::m::User> for UserGQL {
    fn from(user: showtimes_db::m::User) -> Self {
        UserGQL {
            id: user.id,
            username: user.username,
            kind: user.kind,
            api_key: user.api_key,
            registered: user.registered,
            avatar: user.avatar,
            created: user.created,
            updated: user.updated,
            disallow_server_fetch: false,
            requester: None,
        }
    }
}

impl From<&showtimes_db::m::User> for UserGQL {
    fn from(user: &showtimes_db::m::User) -> Self {
        UserGQL {
            id: user.id,
            username: user.username.clone(),
            kind: user.kind,
            api_key: user.api_key.clone(),
            registered: user.registered,
            avatar: user.avatar.clone(),
            created: user.created,
            updated: user.updated,
            disallow_server_fetch: false,
            requester: None,
        }
    }
}

impl From<showtimes_db::m::APIKey> for APIKeyDataGQL {
    fn from(key: showtimes_db::m::APIKey) -> Self {
        APIKeyDataGQL {
            key: APIKeyGQL::from(key.key),
            capabilities: key.capabilities.iter().map(|&c| c.into()).collect(),
        }
    }
}

impl From<&showtimes_db::m::APIKey> for APIKeyDataGQL {
    fn from(key: &showtimes_db::m::APIKey) -> Self {
        APIKeyDataGQL {
            key: APIKeyGQL::from(key.key),
            capabilities: key.capabilities.iter().map(|&c| c.into()).collect(),
        }
    }
}

impl UserGQL {
    /// A GQL context for user with server fetch disabled
    pub fn with_disable_server_fetch(mut self) -> Self {
        self.disallow_server_fetch = true;
        self
    }

    /// Set the requester
    pub fn with_requester(mut self, requester: ServerQueryUser) -> Self {
        self.requester = Some(requester);
        self
    }
}

/// A user session object
#[derive(SimpleObject)]
pub struct UserSessionGQL {
    /// The user object
    user: UserGQL,
    /// The user's session token
    token: String,
    /// Refresh token for the current user session
    ///
    /// This will be available when you login or if you provide
    /// `x-refresh-token` header when you are requesting user information.
    #[graphql(name = "refreshToken")]
    refresh_token: Option<String>,
}

impl UserSessionGQL {
    /// Create a new user session
    pub fn new(user: &showtimes_db::m::User, token: impl Into<String>) -> Self {
        let gql_user = UserGQL::from(user).with_requester(user.into());

        UserSessionGQL {
            user: gql_user,
            token: token.into(),
            refresh_token: None,
        }
    }

    /// Create a new user session with refresh token
    pub fn with_refresh_token(mut self, token: impl Into<String>) -> Self {
        self.refresh_token = Some(token.into());
        self
    }
}
