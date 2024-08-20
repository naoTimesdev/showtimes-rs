use super::{prelude::*, servers::ServerGQL};
use async_graphql::{Enum, Object, SimpleObject};
use futures::TryStreamExt;
use showtimes_db::{mongodb::bson::doc, DatabaseShared};

/// Enum to hold user kinds
#[derive(Enum, Default, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[graphql(remote = "showtimes_db::m::UserKind")]
pub enum UserKindGQL {
    /// A normal user
    #[default]
    User,
    /// An admin user, can see all users and manage all servers
    Admin,
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

    /// The user's API key
    async fn api_key(&self) -> String {
        self.api_key.clone()
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
        #[graphql(desc = "The number of servers to return", name = "perPage")] per_page: Option<
            u32,
        >,
        #[graphql(desc = "The cursor to start from")] cursor: Option<UlidGQL>,
    ) -> async_graphql::Result<PaginatedGQL<ServerGQL>> {
        if self.disallow_server_fetch {
            return Err("Servers fetch from this context is disabled to avoid looping".into());
        }

        let db = ctx.data_unchecked::<DatabaseShared>();

        // Allowed range of per_page is 10-100, with
        let per_page = per_page.filter(|&p| (2..=100).contains(&p)).unwrap_or(20);

        let srv_handler = showtimes_db::ServerHandler::new(db);

        let doc_query = match cursor {
            Some(cursor) => {
                doc! {
                    "owners.id": self.id.to_string(),
                    "id": { "$gte": cursor.to_string() }
                }
            }
            None => doc! { "owners.id": self.id.to_string() },
        };

        let cursor = srv_handler
            .get_collection()
            .find(doc_query)
            .limit((per_page + 1) as i64)
            .sort(doc! { "id": 1 })
            .await?;
        let count = srv_handler
            .get_collection()
            .count_documents(doc! { "owners.id": self.id.to_string() })
            .await?;

        let mut all_servers: Vec<showtimes_db::m::Server> = cursor.try_collect().await?;

        // If all_servers is equal to per_page, then there is a next page
        let last_srv = if all_servers.len() == per_page as usize {
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
        }
    }
}

impl UserGQL {
    pub fn with_disable_server_fetch(mut self) -> Self {
        self.disallow_server_fetch = true;
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
        UserSessionGQL {
            user: user.into(),
            token: token.into(),
        }
    }
}
