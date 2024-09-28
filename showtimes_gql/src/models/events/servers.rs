use async_graphql::{dataloader::DataLoader, ErrorExtensions, Object, SimpleObject};

use crate::{
    data_loader::ServerDataLoader,
    models::{
        prelude::*,
        servers::{ServerGQL, ServerUserGQL},
    },
};

/// A server created event
pub struct ServerCreatedEventDataGQL {
    id: showtimes_shared::ulid::Ulid,
}

#[Object]
impl ServerCreatedEventDataGQL {
    /// The server's ID
    async fn id(&self) -> UlidGQL {
        self.id.into()
    }

    /// The server information
    async fn server(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<ServerGQL> {
        let loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();

        let user = loader.load_one(self.id).await?.ok_or_else(|| {
            async_graphql::Error::new("Server not found")
                .extend_with(|_, e| e.set("id", self.id.to_string()))
        })?;

        let srv_gql = ServerGQL::from(user);
        Ok(srv_gql.with_projects_disabled())
    }
}

/// The data that contains the server updated information
///
/// Used in conjuction with the [`ServerUpdatedEventDataGQL`]
///
/// Not all fields will be present, only the fields that have been updated
#[derive(SimpleObject)]
pub struct ServerUpdatedEventDataContentGQL {
    /// The change in the server name
    name: Option<String>,
    /// The change in the server integrations API key
    integrations: Option<Vec<IntegrationIdGQL>>,
    /// The change in the owners of the server
    kind: Option<Vec<ServerUserGQL>>,
    /// The change in the server avatar
    avatar: Option<ImageMetadataGQL>,
}

impl ServerUpdatedEventDataContentGQL {
    fn new(
        value: &showtimes_events::m::ServerUpdatedDataEvent,
        parent: showtimes_shared::ulid::Ulid,
    ) -> Self {
        Self {
            name: value.name().map(|v| v.to_string()),
            integrations: value
                .integrations()
                .map(|v| v.iter().map(IntegrationIdGQL::from).collect()),
            kind: value.owners().map(|v| {
                v.iter()
                    .map(|u| ServerUserGQL::from_shared(u, parent))
                    .collect()
            }),
            avatar: value.avatar().map(ImageMetadataGQL::from),
        }
    }
}

/// A server updated event
pub struct ServerUpdatedEventDataGQL {
    id: showtimes_shared::ulid::Ulid,
    before: showtimes_events::m::ServerUpdatedDataEvent,
    after: showtimes_events::m::ServerUpdatedDataEvent,
}

#[Object]
impl ServerUpdatedEventDataGQL {
    /// The server's ID
    async fn id(&self) -> UlidGQL {
        self.id.into()
    }

    /// The server information
    async fn server(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<ServerGQL> {
        let loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();

        let user = loader.load_one(self.id).await?.ok_or_else(|| {
            async_graphql::Error::new("Server not found")
                .extend_with(|_, e| e.set("id", self.id.to_string()))
        })?;

        let srv_gql = ServerGQL::from(user);
        Ok(srv_gql.with_projects_disabled())
    }

    /// The server's data before the update
    async fn before(&self) -> ServerUpdatedEventDataContentGQL {
        ServerUpdatedEventDataContentGQL::new(&self.before, self.id)
    }

    /// The server's data after the update
    async fn after(&self) -> ServerUpdatedEventDataContentGQL {
        ServerUpdatedEventDataContentGQL::new(&self.after, self.id)
    }
}

/// A server deleted event
#[derive(SimpleObject)]
pub struct ServerDeletedEventDataGQL {
    /// The server ID that was deleted
    id: UlidGQL,
}

impl From<showtimes_events::m::ServerCreatedEvent> for ServerCreatedEventDataGQL {
    fn from(value: showtimes_events::m::ServerCreatedEvent) -> Self {
        Self {
            id: value.id(),
        }
    }
}

impl From<&showtimes_events::m::ServerCreatedEvent> for ServerCreatedEventDataGQL {
    fn from(value: &showtimes_events::m::ServerCreatedEvent) -> Self {
        Self {
            id: value.id(),
        }
    }
}

impl From<showtimes_events::m::ServerDeletedEvent> for ServerDeletedEventDataGQL {
    fn from(value: showtimes_events::m::ServerDeletedEvent) -> Self {
        Self {
            id: value.id().into(),
        }
    }
}

impl From<&showtimes_events::m::ServerDeletedEvent> for ServerDeletedEventDataGQL {
    fn from(value: &showtimes_events::m::ServerDeletedEvent) -> Self {
        Self {
            id: value.id().into(),
        }
    }
}

impl From<showtimes_events::m::ServerUpdatedEvent> for ServerUpdatedEventDataGQL {
    fn from(value: showtimes_events::m::ServerUpdatedEvent) -> Self {
        Self {
            id: value.id(),
            before: value.before().clone(),
            after: value.after().clone(),
        }
    }
}

impl From<&showtimes_events::m::ServerUpdatedEvent> for ServerUpdatedEventDataGQL {
    fn from(value: &showtimes_events::m::ServerUpdatedEvent) -> Self {
        Self {
            id: value.id(),
            before: value.before().clone(),
            after: value.after().clone(),
        }
    }
}
