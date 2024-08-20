use std::collections::HashMap;

use async_graphql::{dataloader::Loader, FieldError};
use futures::TryStreamExt;
use showtimes_db::{mongodb::bson::doc, DatabaseShared};
use showtimes_shared::ulid::Ulid;

/// A simple data loader for Discord IDs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiscordIdLoad(pub String);

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
