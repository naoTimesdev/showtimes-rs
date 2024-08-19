use super::prelude::*;
use async_graphql::{Object, SimpleObject};

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
        }
    }
}

impl From<&showtimes_db::m::User> for UserGQL {
    fn from(user: &showtimes_db::m::User) -> Self {
        UserGQL {
            id: user.id,
            username: user.username.clone(),
            kind: user.kind.clone(),
            api_key: user.api_key.clone(),
            registered: user.registered,
            avatar: user.avatar.clone(),
            created: user.created,
            updated: user.updated,
        }
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
