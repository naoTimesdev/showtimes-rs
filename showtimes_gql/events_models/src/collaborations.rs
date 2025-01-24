//! A collaboration sync or invite events interactor

use async_graphql::{dataloader::DataLoader, Object, SimpleObject};

use errors::GQLError;
use showtimes_gql_common::{data_loader::ServerSyncLoader, *};
use showtimes_gql_models::collaborations::CollaborationSyncGQL;

/// A collab created or initiation event
pub struct CollabCreatedEventDataGQL {
    id: showtimes_shared::ulid::Ulid,
}

#[Object]
impl CollabCreatedEventDataGQL {
    /// The collab's ID
    async fn id(&self) -> UlidGQL {
        self.id.into()
    }

    /// The collab information
    async fn collab(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<CollaborationSyncGQL> {
        let loader = ctx.data_unchecked::<DataLoader<ServerSyncLoader>>();

        let item = loader.load_one(self.id).await?.ok_or_else(|| {
            GQLError::new("Collab not found", GQLErrorCode::ServerSyncNotFound)
                .extend(|e| e.set("id", self.id.to_string()))
        })?;

        let gql_item = CollaborationSyncGQL::from(item);
        Ok(gql_item)
    }
}

/// A collab acceptance event
pub struct CollabAcceptedEventDataGQL {
    id: showtimes_shared::ulid::Ulid,
    sync_id: showtimes_shared::ulid::Ulid,
}

#[Object]
impl CollabAcceptedEventDataGQL {
    /// The collab's ID
    async fn id(&self) -> UlidGQL {
        self.sync_id.into()
    }

    /// The invite ID, this ID has been deleted from database
    #[graphql(name = "inviteId")]
    async fn invite_id(&self) -> UlidGQL {
        self.id.into()
    }

    /// The collab information
    async fn collab(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<CollaborationSyncGQL> {
        let loader = ctx.data_unchecked::<DataLoader<ServerSyncLoader>>();

        let item = loader.load_one(self.id).await?.ok_or_else(|| {
            GQLError::new("Collab not found", GQLErrorCode::ServerSyncNotFound)
                .extend(|e| e.set("id", self.id.to_string()))
        })?;

        let gql_item = CollaborationSyncGQL::from(item);
        Ok(gql_item)
    }
}

/// A collab rejected event, this is done from the invitee side
#[derive(SimpleObject)]
pub struct CollabRejectedEventDataGQL {
    /// The collab/invite ID that was rejected
    id: UlidGQL,
}

/// A collab retracted event, this is done from the inviter side
#[derive(SimpleObject)]
pub struct CollabRetractedEventDataGQL {
    /// The collab/invite ID that was retracted
    id: UlidGQL,
}

/// The sync target that was deleted, this is just a mini information
/// without the full server and project information.
#[derive(SimpleObject)]
pub struct CollabDeletedEventDataSyncTargetGQL {
    server: UlidGQL,
    project: UlidGQL,
}

/// A collab deleted or unlinked event
pub struct CollabDeletedEventDataGQL {
    id: showtimes_shared::ulid::Ulid,
    target: showtimes_db::m::ServerCollaborationSyncTarget,
    is_deleted: bool,
}

#[Object]
impl CollabDeletedEventDataGQL {
    /// The collab's ID
    async fn id(&self) -> UlidGQL {
        self.id.into()
    }

    /// The target sync that was deleted
    async fn target(&self) -> CollabDeletedEventDataSyncTargetGQL {
        CollabDeletedEventDataSyncTargetGQL {
            server: self.target.server.into(),
            project: self.target.project.into(),
        }
    }

    /// The collab information, when the unlink/delete event occurs,
    /// If there is only one of the synced server left, the collab will be deleted
    /// entirely from the database, otherwise, only the sync target will be deleted
    async fn collab(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Option<CollaborationSyncGQL>> {
        if self.is_deleted {
            return Ok(None);
        }
        let loader = ctx.data_unchecked::<DataLoader<ServerSyncLoader>>();

        let item = loader.load_one(self.id).await?.ok_or_else(|| {
            GQLError::new("Collab not found", GQLErrorCode::ServerSyncNotFound)
                .extend(|e| e.set("id", self.id.to_string()))
        })?;

        let gql_item = CollaborationSyncGQL::from(item);
        Ok(Some(gql_item))
    }
}

impl From<showtimes_events::m::CollabCreatedEvent> for CollabCreatedEventDataGQL {
    fn from(value: showtimes_events::m::CollabCreatedEvent) -> Self {
        Self { id: value.id() }
    }
}

impl From<&showtimes_events::m::CollabCreatedEvent> for CollabCreatedEventDataGQL {
    fn from(value: &showtimes_events::m::CollabCreatedEvent) -> Self {
        Self { id: value.id() }
    }
}

impl From<showtimes_events::m::CollabAcceptedEvent> for CollabAcceptedEventDataGQL {
    fn from(value: showtimes_events::m::CollabAcceptedEvent) -> Self {
        Self {
            id: value.id(),
            sync_id: value.sync_id(),
        }
    }
}

impl From<&showtimes_events::m::CollabAcceptedEvent> for CollabAcceptedEventDataGQL {
    fn from(value: &showtimes_events::m::CollabAcceptedEvent) -> Self {
        Self {
            id: value.id(),
            sync_id: value.sync_id(),
        }
    }
}

impl From<showtimes_events::m::CollabRejectedEvent> for CollabRejectedEventDataGQL {
    fn from(value: showtimes_events::m::CollabRejectedEvent) -> Self {
        Self {
            id: value.id().into(),
        }
    }
}

impl From<&showtimes_events::m::CollabRejectedEvent> for CollabRejectedEventDataGQL {
    fn from(value: &showtimes_events::m::CollabRejectedEvent) -> Self {
        Self {
            id: value.id().into(),
        }
    }
}

impl From<showtimes_events::m::CollabRetractedEvent> for CollabRetractedEventDataGQL {
    fn from(value: showtimes_events::m::CollabRetractedEvent) -> Self {
        Self {
            id: value.id().into(),
        }
    }
}

impl From<&showtimes_events::m::CollabRetractedEvent> for CollabRetractedEventDataGQL {
    fn from(value: &showtimes_events::m::CollabRetractedEvent) -> Self {
        Self {
            id: value.id().into(),
        }
    }
}

impl From<showtimes_events::m::CollabDeletedEvent> for CollabDeletedEventDataGQL {
    fn from(value: showtimes_events::m::CollabDeletedEvent) -> Self {
        Self {
            id: value.id(),
            target: value.target(),
            is_deleted: value.is_deleted(),
        }
    }
}

impl From<&showtimes_events::m::CollabDeletedEvent> for CollabDeletedEventDataGQL {
    fn from(value: &showtimes_events::m::CollabDeletedEvent) -> Self {
        Self {
            id: value.id(),
            target: value.target(),
            is_deleted: value.is_deleted(),
        }
    }
}
