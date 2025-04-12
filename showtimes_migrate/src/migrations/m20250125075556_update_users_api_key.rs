use futures_util::TryStreamExt;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use showtimes_db::{ClientShared, DatabaseShared, m::ShowModelHandler};
use showtimes_shared::ulid_serializer;

use crate::common::env_or_exit;

use super::Migration;

pub struct M20250125075556UpdateUsersApiKey {
    client: ClientShared,
    db: DatabaseShared,
}

#[async_trait::async_trait]
impl Migration for M20250125075556UpdateUsersApiKey {
    fn init(client: &ClientShared, db: &DatabaseShared) -> Self {
        Self {
            client: client.clone(),
            db: db.clone(),
        }
    }

    fn name(&self) -> &'static str {
        "M20250125075556UpdateUsersApiKey"
    }

    fn timestamp(&self) -> jiff::Timestamp {
        jiff::civil::datetime(2025, 1, 25, 7, 55, 56, 0)
            .to_zoned(jiff::tz::TimeZone::UTC)
            .unwrap()
            .timestamp()
    }

    fn clone_box(&self) -> Box<dyn Migration> {
        Box::new(Self {
            client: self.client.clone(),
            db: self.db.clone(),
        })
    }

    async fn up(&self) -> anyhow::Result<()> {
        let meili_url = env_or_exit("MEILI_URL");
        let meili_key = env_or_exit("MEILI_KEY");

        tracing::info!("Setting up database connection...");
        let collection = self
            .db
            .collection::<UserV1>(showtimes_db::m::User::collection_name());

        let users_db = showtimes_db::UserHandler::new(&self.db);

        tracing::info!("Creating Meilisearch client instances...");
        let meilisearch = showtimes_search::create_connection(&meili_url, &meili_key).await?;
        let s_user_index = meilisearch.index(showtimes_search::models::User::index_name());
        let s_user_pk = showtimes_search::models::User::primary_key();

        // Get all users with api key as string
        tracing::info!("Updating all users");
        let mut cursor = collection
            .find(doc! {
                "api_key": {
                    "$type": "string"
                }
            })
            .await?;
        let mut queued_search = vec![];
        while let Some(user) = cursor.try_next().await? {
            let mut new_user = showtimes_db::m::User::from(user);

            tracing::info!("Updating user: {}", new_user.id);
            users_db.save_direct(&mut new_user, None).await?;

            // Update the user
            queued_search.push(showtimes_search::models::User::from(new_user));
        }

        tracing::info!("Updating users search index...");
        if !queued_search.is_empty() {
            let task = s_user_index
                .add_or_update(&queued_search, Some(s_user_pk))
                .await?;

            tracing::info!(" Waiting for users index update to complete...");
            task.wait_for_completion(&*meilisearch, None, None).await?;
        }
        tracing::info!("Migration completed successfully");

        Ok(())
    }

    async fn down(&self) -> anyhow::Result<()> {
        // Ignore this migration
        Ok(())
    }
}

/// The old user object
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserV1 {
    /// The user's ID
    #[serde(with = "ulid_serializer", default = "ulid_serializer::default")]
    id: showtimes_shared::ulid::Ulid,
    /// The user's username
    ///
    /// This can be changed by the user.
    username: String,
    /// The user's avatar
    ///
    /// This can be changed by the user.
    avatar: Option<showtimes_db::m::ImageMetadata>,
    /// The user API key
    api_key: showtimes_shared::APIKey,
    /// The user kind
    kind: showtimes_db::m::UserKind,
    /// The user discord information
    discord_meta: showtimes_db::m::DiscordUser,
    /// Check if the user registered, this is used to verify
    /// data from old migrations
    registered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<mongodb::bson::oid::ObjectId>,
    #[serde(
        with = "jiff::fmt::serde::timestamp::second::required",
        default = "jiff::Timestamp::now"
    )]
    created: jiff::Timestamp,
    #[serde(
        with = "jiff::fmt::serde::timestamp::second::required",
        default = "jiff::Timestamp::now"
    )]
    updated: jiff::Timestamp,
}

impl From<UserV1> for showtimes_db::m::User {
    fn from(user: UserV1) -> Self {
        let mut stub = showtimes_db::m::User::stub();
        stub.id = user.id;
        stub.username = user.username;
        stub.avatar = user.avatar;
        stub.discord_meta = user.discord_meta;
        stub.registered = user.registered;
        stub.created = user.created;
        stub.updated = user.updated;
        if let Some(id) = user._id {
            stub.set_id(id);
        }

        // Default should give all capabilities
        let new_api_keys = showtimes_db::m::APIKey::new(
            user.api_key,
            showtimes_db::m::APIKeyCapability::all().to_vec(),
        );
        stub.api_key = vec![new_api_keys];

        stub
    }
}
