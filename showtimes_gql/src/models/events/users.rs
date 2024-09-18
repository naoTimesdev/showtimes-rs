use async_graphql::{dataloader::DataLoader, ErrorExtensions, Object, SimpleObject};

use crate::{
    data_loader::UserDataLoader,
    models::{
        prelude::*,
        users::{UserGQL, UserKindGQL},
    },
    queries::ServerQueryUser,
};

/// A user created event
pub struct UserCreatedEventDataGQL {
    id: showtimes_shared::ulid::Ulid,
    requester: ServerQueryUser,
}

#[Object]
impl UserCreatedEventDataGQL {
    /// The user's ID
    async fn id(&self) -> UlidGQL {
        self.id.into()
    }

    /// The user information
    async fn user(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<UserGQL> {
        let loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();

        let user = loader.load_one(self.id).await?.ok_or_else(|| {
            async_graphql::Error::new("User not found")
                .extend_with(|_, e| e.set("id", self.id.to_string()))
        })?;

        let user_gql = UserGQL::from(user);
        Ok(user_gql
            .with_disable_server_fetch()
            .with_requester(self.requester))
    }
}

/// The data that contains the user's updated information
///
/// Used in conjuction with the [`UserUpdatedEventDataGQL`]
///
/// Not all fields will be present, only the fields that have been updated
#[derive(SimpleObject)]
pub struct UserUpdatedEventDataContentGQL {
    /// The change in the user's name
    name: Option<String>,
    /// The change in the user's API key
    #[graphql(name = "apiKey")]
    api_key: Option<APIKeyGQL>,
    /// The change in the user's kind
    kind: Option<UserKindGQL>,
    /// The change in the user's avatar
    avatar: Option<ImageMetadataGQL>,
    /// The change in the user's discord meta
    ///
    /// This is just a boolean to prevent leaking sensitive information
    #[graphql(name = "discordUpdated")]
    discord_updated: bool,
}

impl From<showtimes_events::m::UserUpdatedDataEvent> for UserUpdatedEventDataContentGQL {
    fn from(value: showtimes_events::m::UserUpdatedDataEvent) -> Self {
        Self {
            name: value.name().map(|n| n.to_string()),
            api_key: value.api_key().map(|a| a.into()),
            kind: value.kind().map(|k| k.into()),
            avatar: value.avatar().map(|a| a.into()),
            discord_updated: value.discord_meta().is_some(),
        }
    }
}

impl From<&showtimes_events::m::UserUpdatedDataEvent> for UserUpdatedEventDataContentGQL {
    fn from(value: &showtimes_events::m::UserUpdatedDataEvent) -> Self {
        Self {
            name: value.name().map(|n| n.to_string()),
            api_key: value.api_key().map(|a| a.into()),
            kind: value.kind().map(|k| k.into()),
            avatar: value.avatar().map(|a| a.into()),
            discord_updated: value.discord_meta().is_some(),
        }
    }
}

/// A user updated event
pub struct UserUpdatedEventDataGQL {
    id: showtimes_shared::ulid::Ulid,
    before: showtimes_events::m::UserUpdatedDataEvent,
    after: showtimes_events::m::UserUpdatedDataEvent,
    requester: ServerQueryUser,
}

#[Object]
impl UserUpdatedEventDataGQL {
    /// The user's ID
    async fn id(&self) -> UlidGQL {
        self.id.into()
    }

    /// The user information
    async fn user(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<UserGQL> {
        let loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();

        let user = loader.load_one(self.id).await?.ok_or_else(|| {
            async_graphql::Error::new("User not found")
                .extend_with(|_, e| e.set("id", self.id.to_string()))
        })?;

        let user_gql = UserGQL::from(user);
        Ok(user_gql
            .with_disable_server_fetch()
            .with_requester(self.requester))
    }

    /// The user's data before the update
    async fn before(&self) -> UserUpdatedEventDataContentGQL {
        UserUpdatedEventDataContentGQL::from(&self.before)
    }

    /// The user's data after the update
    async fn after(&self) -> UserUpdatedEventDataContentGQL {
        UserUpdatedEventDataContentGQL::from(&self.after)
    }
}

impl UserCreatedEventDataGQL {
    pub fn new(event: &showtimes_events::m::UserCreatedEvent, requester: ServerQueryUser) -> Self {
        Self {
            id: event.id(),
            requester,
        }
    }
}

impl UserUpdatedEventDataGQL {
    pub fn new(event: &showtimes_events::m::UserUpdatedEvent, requester: ServerQueryUser) -> Self {
        Self {
            id: event.id(),
            before: event.before().clone(),
            after: event.after().clone(),
            requester,
        }
    }
}
