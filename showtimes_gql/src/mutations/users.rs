use std::sync::Arc;

use async_graphql::{dataloader::DataLoader, Error, ErrorExtensions, InputObject, Upload};
use showtimes_db::{m::UserKind, DatabaseShared, UserHandler};
use showtimes_fs::FsPool;
use showtimes_search::SearchClientShared;
use tokio::io::AsyncSeekExt;

use crate::{
    data_loader::UserDataLoader,
    models::users::{UserGQL, UserKindGQL},
};

/// The user input object on what to update
///
/// All fields are optional
#[derive(InputObject)]
pub struct UserInputGQL {
    /// The user's username
    #[graphql(validator(min_length = 5, max_length = 128))]
    username: Option<String>,
    /// The user's kind
    ///
    /// This could only work if you're an Admin or Owner
    ///
    /// The following restriction is applied:
    /// - User -> Admin, with user auth: No
    /// - Admin -> User, with user auth: Yes
    /// - Any -> Owner, with any auth: No
    /// - Owner -> Any, with any auth: Yes
    kind: Option<UserKindGQL>,
    /// Reset the API key
    #[graphql(name = "resetApiKey")]
    reset_api_key: Option<bool>,
    /// The user's avatar
    avatar: Option<Upload>,
}

impl UserInputGQL {
    /// Check if any field is set
    fn is_any_set(&self) -> bool {
        self.username.is_some()
            || self.kind.is_some()
            || self.reset_api_key.is_some()
            || self.avatar.is_some()
    }
}

/// The user who requested the update
pub struct UserRequester {
    /// Specific user specified by ULID
    id: Option<showtimes_shared::ulid::Ulid>,
    requester: showtimes_db::m::User,
}

impl UserRequester {
    pub fn new(requester: showtimes_db::m::User) -> Self {
        Self {
            id: None,
            requester,
        }
    }

    pub fn with_id(self, id: showtimes_shared::ulid::Ulid) -> Self {
        Self {
            id: Some(id),
            requester: self.requester,
        }
    }
}

pub async fn mutate_users_update(
    ctx: &async_graphql::Context<'_>,
    user: UserRequester,
    input: UserInputGQL,
) -> async_graphql::Result<UserGQL> {
    if !input.is_any_set() {
        return Err(Error::new("No fields to update"));
    }

    let loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();
    let db = ctx.data_unchecked::<DatabaseShared>();
    let storages = ctx.data_unchecked::<Arc<FsPool>>();
    let meili = ctx.data_unchecked::<SearchClientShared>();

    let user_info = match user.id {
        Some(id) => loader.load_one(id).await?.ok_or_else(|| {
            Error::new("User not found").extend_with(|_, e| {
                e.set("id", id.to_string());
                e.set("reason", "invalid_user");
            })
        })?,
        None => user.requester.clone(),
    };

    if user_info.kind == UserKind::Owner {
        // Fails, Owner cannot be updated
        return Err(Error::new("Owner cannot be updated"));
    }

    if user.requester.kind == UserKind::User && user_info.id != user.requester.id {
        // Fails, User cannot be updated by another user
        return Err(Error::new("User cannot be updated by another user"));
    }

    let proceed_user_kind = match (user_info.kind, input.kind) {
        // Disallow User -> Admin
        (UserKind::User, Some(UserKindGQL::Admin)) => {
            // As long as the requester is Owner, it's okay
            user.requester.kind == UserKind::Owner
        }
        // Disallow Any -> Owner
        (_, Some(UserKindGQL::Owner)) => false,
        // Allow Admin -> User
        (UserKind::Admin, Some(UserKindGQL::User)) => {
            // As long as the requester is Owner, it's okay
            user.requester.kind == UserKind::Owner
        }
        // Disallow Owner -> Any
        (UserKind::Owner, _) => false,
        (_, _) => true,
    };

    if !proceed_user_kind {
        return Err(Error::new("Invalid user kind update").extend_with(|_, e| {
            e.set("reason", "invalid_user_kind");
            e.set("from", user_info.kind.to_name());
            e.set(
                "to",
                match input.kind {
                    Some(data) => data.to_name(),
                    None => "None",
                },
            );
        }));
    }

    let mut user_info = user_info.clone();
    if let Some(username) = input.username {
        user_info.username = username;
    }
    if let Some(kind) = input.kind {
        user_info.kind = kind.into();
    }
    if let Some(true) = input.reset_api_key {
        user_info.api_key = showtimes_shared::APIKey::new();
    }
    if let Some(avatar_upload) = input.avatar {
        let info_up = avatar_upload.value(ctx)?;
        let mut file_target = tokio::fs::File::from_std(info_up.content);

        // Get format
        let format = crate::image::detect_upload_data(&mut file_target).await?;
        // Seek back to the start of the file
        file_target.seek(std::io::SeekFrom::Start(0)).await?;

        let filename = format!("avatar.{}", format.as_extension());

        storages
            .file_stream_upload(
                user_info.id.to_string(),
                &filename,
                &mut file_target,
                None,
                Some(showtimes_fs::FsFileKind::Images),
            )
            .await?;

        let image_meta = showtimes_db::m::ImageMetadata::new(
            showtimes_fs::FsFileKind::Images.as_path_name(),
            user_info.id,
            &filename,
            format.as_extension(),
            None::<String>,
        );

        user_info.avatar = Some(image_meta);
    }

    // Update the user
    let user_handler = UserHandler::new(db);
    user_handler.save(&mut user_info, None).await?;

    // Update index
    let user_search = showtimes_search::models::User::from(user_info.clone());
    user_search.update_document(meili).await?;

    let user_gql: UserGQL = user_info.into();

    Ok(user_gql.with_requester(user.requester.into()))
}
