use crate::queries::ServerQueryUser;

use super::{prelude::*, servers::ServerGQL};
use async_graphql::{Enum, Object, SimpleObject};
use showtimes_db::mongodb::bson::doc;

/// Enum to hold user kinds
#[derive(Enum, Default, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[graphql(remote = "showtimes_db::m::UserKind")]
pub enum UserKindGQL {
    /// A normal user
    #[default]
    User,
    /// An admin user, can see all users and manage all servers
    Admin,
    /// "Owner" user, basically can do anything
    Owner,
}

/// The main user object
pub struct UserGQL {
    id: showtimes_shared::ulid::Ulid,
    username: String,
    kind: showtimes_db::m::UserKind,
    api_key: String,
    registered: bool,
    avatar: Option<showtimes_db::m::ImageMetadata>,
    created: chrono::DateTime<chrono::Utc>,
    updated: chrono::DateTime<chrono::Utc>,
    disallow_server_fetch: bool,
    requester: Option<ServerQueryUser>,
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
    async fn api_key(&self) -> Option<String> {
        if let Some(requester) = self.requester {
            // Only return the API key if the requester is the same user or the requester is not a user
            if requester.id() == self.id || requester.kind() != showtimes_db::m::UserKind::User {
                Some(self.api_key.clone())
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
    async fn servers(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Specify server IDs to query")] ids: Option<
            Vec<crate::models::prelude::UlidGQL>,
        >,
        #[graphql(
            name = "perPage",
            desc = "The number of servers to return, default to 20",
            validator(minimum = 2, maximum = 100)
        )]
        per_page: Option<u32>,
        #[graphql(desc = "The cursor to start from")] cursor: Option<
            crate::models::prelude::UlidGQL,
        >,
    ) -> async_graphql::Result<PaginatedGQL<ServerGQL>> {
        if self.disallow_server_fetch {
            return Err("Servers fetch from this context is disabled to avoid looping".into());
        }

        let mut queries = crate::queries::servers::ServerQuery::new()
            .with_current_user(crate::queries::ServerQueryUser::new(self.id, self.kind));
        if let Some(ids) = ids {
            queries.set_ids(ids.into_iter().map(|id| *id).collect());
        };
        if let Some(per_page) = per_page {
            queries.set_per_page(per_page);
        }
        if let Some(cursor) = cursor {
            queries.set_cursor(*cursor);
        }

        let results = crate::queries::servers::query_servers_paginated(ctx, queries).await?;

        Ok(results)
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

impl UserGQL {
    pub fn with_disable_server_fetch(mut self) -> Self {
        self.disallow_server_fetch = true;
        self
    }

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
}

impl UserSessionGQL {
    /// Create a new user session
    pub fn new(user: showtimes_db::m::User, token: impl Into<String>) -> Self {
        let gql_user = UserGQL::from(&user).with_requester(user.into());

        UserSessionGQL {
            user: gql_user,
            token: token.into(),
        }
    }
}
