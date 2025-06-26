use std::sync::Arc;

use async_graphql::{InputObject, Upload, dataloader::DataLoader};
use showtimes_db::{DatabaseShared, UserHandler, m::UserKind};
use showtimes_fs::FsPool;
use showtimes_search::SearchClientShared;
use showtimes_session::{manager::SharedSessionManager, oauth2::discord::DiscordClient};
use tokio::io::AsyncSeekExt;

use showtimes_gql_common::{
    APIKeyCapabilityGQL, APIKeyGQL, GQLErrorCode, GQLErrorExt, UserKindGQL,
    data_loader::{DiscordIdLoad, UserDataLoader},
    errors::GQLError,
};
use showtimes_gql_models::users::{APIKeyDataGQL, UserGQL, UserSessionGQL};

use crate::{execute_search_events, is_string_set};

/// The input information of an API key
#[derive(InputObject)]
pub struct UserAPIKeyInputGQL {
    /// The current API key to be reset or match with
    ///
    /// If not provided, we're assuming we're creating a new key
    /// so make sure to set `capabilities`
    key: Option<APIKeyGQL>,
    /// Reset the API key
    reset: Option<bool>,
    /// Remove the API key
    remove: Option<bool>,
    /// The privilege level of the API key
    #[graphql(validator(min_items = 1))]
    capabilities: Option<Vec<APIKeyCapabilityGQL>>,
}

impl UserAPIKeyInputGQL {
    /// Check if any field is set
    fn is_any_set(&self) -> bool {
        self.reset.is_some_and(|d| d)
            || self.capabilities.is_some()
            || self.key.is_some()
            || self.remove.is_some_and(|d| d)
    }

    fn capabilities(&self) -> Option<Vec<showtimes_db::m::APIKeyCapability>> {
        self.capabilities.as_ref().map(|capabilities| {
            capabilities
                .iter()
                .map(|d| (*d).into())
                .collect::<Vec<showtimes_db::m::APIKeyCapability>>()
        })
    }

    fn dump_query(
        &self,
        ctx: &mut async_graphql::indexmap::IndexMap<async_graphql::Name, async_graphql::Value>,
    ) {
        if let Some(key) = &self.key {
            ctx.insert(
                async_graphql::Name::new("key"),
                async_graphql::Value::String(key.to_string()),
            );
        }
        if let Some(reset) = self.reset {
            ctx.insert(
                async_graphql::Name::new("reset"),
                async_graphql::Value::Boolean(reset),
            );
        }
        if let Some(capabilities) = &self.capabilities {
            let capabilities_map = capabilities
                .iter()
                .map(|d| async_graphql::Value::String(d.to_name().to_string()))
                .collect::<Vec<async_graphql::Value>>();
            ctx.insert(
                async_graphql::Name::new("capabilities"),
                async_graphql::Value::List(capabilities_map),
            );
        }
    }
}

/// The user input object on what to update
///
/// All fields are optional
#[derive(InputObject)]
pub struct UserInputGQL {
    /// The user's username
    #[graphql(validator(min_length = 3, max_length = 128))]
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
    /// The user's avatar
    avatar: Option<Upload>,
    /// Modify the user's API keys
    api_keys: Option<Vec<UserAPIKeyInputGQL>>,
}

impl UserInputGQL {
    /// Check if any field is set
    fn is_any_set(&self) -> bool {
        let is_api_set = self
            .api_keys
            .as_ref()
            .is_some_and(|d| d.iter().any(|d| d.is_any_set()));
        is_string_set(&self.username) || self.kind.is_some() || self.avatar.is_some() || is_api_set
    }

    fn dump_query(&self, ctx: &mut async_graphql::ErrorExtensionValues) {
        if let Some(username) = &self.username {
            ctx.set("username", username.to_string());
        }
        if let Some(kind) = &self.kind {
            ctx.set("kind", kind.to_name());
        }
        ctx.set("avatar_change", self.avatar.is_some());
        if let Some(api_keys) = &self.api_keys {
            ctx.set(
                "api_keys",
                api_keys
                    .iter()
                    .map(|d| {
                        let mut f_new = async_graphql::indexmap::IndexMap::new();
                        d.dump_query(&mut f_new);
                        async_graphql::Value::Object(f_new)
                    })
                    .collect::<Vec<async_graphql::Value>>(),
            );
        }
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
        return GQLError::new("No fields to update", GQLErrorCode::MissingModification).into();
    }

    let loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();
    let db = ctx.data_unchecked::<DatabaseShared>();
    let storages = ctx.data_unchecked::<Arc<FsPool>>();
    let meili = ctx.data_unchecked::<SearchClientShared>();

    let user_info = match user.id {
        Some(id) => loader.load_one(id).await?.ok_or_else(|| {
            GQLError::new("User not found", GQLErrorCode::UserNotFound)
                .extend(|e| e.set("id", id.to_string()))
        })?,
        None => user.requester.clone(),
    };

    if user_info.kind == UserKind::Owner {
        // Fails, Owner cannot be updated
        return GQLError::new(
            "Unable to update owner information",
            GQLErrorCode::UserSuperuserMode,
        )
        .extend(|e| {
            e.set("id", user_info.id.to_string());
        })
        .into();
    }

    if user.requester.kind == UserKind::User && user_info.id != user.requester.id {
        // Fails, User cannot be updated by another user
        return GQLError::new(
            "Unable to update user information",
            GQLErrorCode::UserUnauthorized,
        )
        .extend(|e| {
            e.set("id", user_info.id.to_string());
            e.set("requester", user.requester.id.to_string());
        })
        .into();
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
        return GQLError::new(
            "Insufficient privilege to update user privilege/kind",
            GQLErrorCode::UserInsufficientPrivilege,
        )
        .extend(|e| {
            e.set("id", user_info.id.to_string());
            e.set("from", user_info.kind.to_name());
            e.set(
                "to",
                match input.kind {
                    Some(data) => data.to_name(),
                    None => "None",
                },
            );
        })
        .into();
    }

    let mut user_info = user_info.clone();
    let mut user_before = showtimes_events::m::UserUpdatedDataEvent::default();
    let mut user_after = showtimes_events::m::UserUpdatedDataEvent::default();

    if let Some(username) = &input.username {
        user_before.set_name(&user_info.username);
        user_info.username = username.to_string();
        user_after.set_name(&user_info.username);
    }

    if let Some(kind) = input.kind {
        user_before.set_kind(user_info.kind);
        user_info.kind = kind.into();
        user_after.set_kind(user_info.kind);
    }

    // Update API keys
    if let Some(api_keys) = &input.api_keys {
        // set before
        user_before.set_api_key(&user_info.api_key);

        let mut any_api_changes = false;

        for api_key in api_keys {
            match &api_key.key {
                Some(key) => {
                    // find
                    let matched = user_info.api_key.iter_mut().find(|d| d.key == **key);
                    if let Some(matched) = matched {
                        if let Some(remove) = api_key.remove {
                            if remove {
                                let current_len = user_info.api_key.len();
                                user_info.api_key.retain(|d| d.key != **key);
                                if current_len != user_info.api_key.len() {
                                    any_api_changes = true;
                                }
                                // Jump!
                                continue;
                            }
                        }

                        // Reset API key
                        if let Some(reset) = api_key.reset {
                            if reset {
                                matched.update_key(showtimes_shared::APIKey::new());
                                any_api_changes = true;
                            }
                        }
                        if let Some(capabilities) = &api_key.capabilities() {
                            matched.capabilities = capabilities.to_vec();
                            any_api_changes = true;
                        }
                    }
                }
                None => {
                    // create, check
                    let capabilities: Vec<showtimes_db::m::APIKeyCapability> = api_key
                        .capabilities()
                        .unwrap_or_else(|| showtimes_db::m::APIKeyCapability::all().to_vec());

                    let key = showtimes_shared::APIKey::new();
                    user_info
                        .api_key
                        .push(showtimes_db::m::APIKey::new(key, capabilities));
                    any_api_changes = true;
                }
            }
        }

        // If any API key changed, update
        if any_api_changes {
            user_after.set_api_key(&user_info.api_key);
        } else {
            // flush before
            user_before.clear_api_key();
        }
    }

    // Update avatar
    if let Some(avatar_upload) = input.avatar {
        let info_up = avatar_upload.value(ctx).map_err(|err| {
            GQLError::new(
                format!("Failed to read image upload: {err}"),
                GQLErrorCode::IOError,
            )
            .extend(|e| {
                e.set("id", user_info.id.to_string());
                e.set("where", "user");
                e.set("original", format!("{err}"));
                e.set("original_code", format!("{}", err.kind()));
            })
        })?;
        let mut file_target = tokio::fs::File::from_std(info_up.content);

        // Get format
        let format = showtimes_gql_common::image::detect_upload_data(&mut file_target)
            .await
            .map_err(|err| {
                GQLError::new(
                    format!("Failed to detect image format: {err}"),
                    GQLErrorCode::IOError,
                )
                .extend(|e| {
                    e.set("id", user_info.id.to_string());
                    e.set("where", "user");
                    e.set("original", format!("{err}"));
                    e.set("original_code", format!("{}", err.kind()));
                })
            })?;
        // Seek back to the start of the file
        file_target
            .seek(std::io::SeekFrom::Start(0))
            .await
            .map_err(|err| {
                GQLError::new(
                    format!("Failed to seek to image to start: {err}"),
                    GQLErrorCode::IOError,
                )
                .extend(|e| {
                    e.set("id", user_info.id.to_string());
                    e.set("where", "user");
                    e.set("original", format!("{err}"));
                    e.set("original_code", format!("{}", err.kind()));
                })
            })?;

        let filename = format!("avatar.{}", format.as_extension());

        storages
            .file_stream_upload(
                user_info.id.to_string(),
                &filename,
                file_target,
                None,
                Some(showtimes_fs::FsFileKind::Images),
            )
            .await
            .map_err(|err| {
                GQLError::new(
                    format!("Failed to upload image: {err}"),
                    GQLErrorCode::ImageUploadError,
                )
                .extend(|e| {
                    e.set("id", user_info.id.to_string());
                    e.set("where", "user");
                    e.set("original", format!("{err}"));
                })
            })?;

        let image_meta = showtimes_db::m::ImageMetadata::new(
            showtimes_fs::FsFileKind::Images.to_name(),
            user_info.id,
            &filename,
            format.as_extension(),
            None::<String>,
        );

        if let Some(avatar) = &user_info.avatar {
            user_before.set_avatar(avatar);
        }
        user_after.set_avatar(&image_meta);
        user_info.avatar = Some(image_meta);
    }

    // Update the user
    let user_handler = UserHandler::new(db);
    user_handler.save(&mut user_info, None).await.extend_error(
        GQLErrorCode::UserUpdateError,
        |f_m| {
            f_m.set("id", user_info.id.to_string());
            f_m.set("actor", user.requester.id.to_string());
            input.dump_query(f_m);
        },
    )?;

    let search_arc = meili.clone();
    let user_clone = user_info.clone();
    let task_search = tokio::task::spawn(async move {
        let user_search = showtimes_search::models::User::from(user_clone);
        user_search.update_document(&search_arc).await
    });
    let task_events = ctx
        .data_unchecked::<showtimes_events::SharedSHClickHouse>()
        .create_event_async(
            showtimes_events::m::EventKind::UserUpdated,
            showtimes_events::m::UserUpdatedEvent::new(user_info.id, user_before, user_after),
            if user.requester.kind == UserKind::Owner {
                None
            } else {
                Some(user.requester.id.to_string())
            },
        );

    execute_search_events(task_search, task_events).await?;

    let user_gql: UserGQL = user_info.into();

    Ok(user_gql.with_requester(user.requester.into()))
}

pub async fn mutate_users_create_api_key(
    ctx: &async_graphql::Context<'_>,
    user: UserRequester,
    capabilities: Option<Vec<APIKeyCapabilityGQL>>,
) -> async_graphql::Result<APIKeyDataGQL> {
    let remapped_caps = match capabilities {
        Some(capabilities) => {
            if capabilities.is_empty() {
                return GQLError::new("Capabilities cannot be empty", GQLErrorCode::InvalidRequest)
                    .into();
            }

            capabilities
                .iter()
                .map(|d| (*d).into())
                .collect::<Vec<showtimes_db::m::APIKeyCapability>>()
        }
        _ => showtimes_db::m::APIKeyCapability::queries().to_vec(),
    };

    let loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();
    let db = ctx.data_unchecked::<DatabaseShared>();
    let meili = ctx.data_unchecked::<SearchClientShared>();

    let user_info = match user.id {
        Some(id) => loader.load_one(id).await?.ok_or_else(|| {
            GQLError::new("User not found", GQLErrorCode::UserNotFound)
                .extend(|e| e.set("id", id.to_string()))
        })?,
        None => user.requester.clone(),
    };

    if user_info.kind == UserKind::Owner {
        // Fails, Owner cannot be updated
        return GQLError::new(
            "Unable to update owner information",
            GQLErrorCode::UserSuperuserMode,
        )
        .extend(|e| {
            e.set("id", user_info.id.to_string());
        })
        .into();
    }

    if user.requester.kind == UserKind::User && user_info.id != user.requester.id {
        // Fails, User cannot be updated by another user
        return GQLError::new(
            "Unable to update user information",
            GQLErrorCode::UserUnauthorized,
        )
        .extend(|e| {
            e.set("id", user_info.id.to_string());
            e.set("requester", user.requester.id.to_string());
        })
        .into();
    }

    let mut user_info = user_info.clone();

    let mut user_before = showtimes_events::m::UserUpdatedDataEvent::default();
    let mut user_after = showtimes_events::m::UserUpdatedDataEvent::default();

    user_before.set_api_key(&user_info.api_key);

    // generate new key
    let new_key = showtimes_shared::APIKey::new();
    user_info
        .api_key
        .push(showtimes_db::m::APIKey::new(new_key, remapped_caps.clone()));
    user_after.set_api_key(&user_info.api_key);

    // Update the user
    let user_handler = UserHandler::new(db);
    user_handler.save(&mut user_info, None).await.extend_error(
        GQLErrorCode::UserUpdateError,
        |f_m| {
            f_m.set("id", user_info.id.to_string());
            f_m.set("actor", user.requester.id.to_string());
        },
    )?;

    let search_arc = meili.clone();
    let user_clone = user_info.clone();
    let task_search = tokio::task::spawn(async move {
        let user_search = showtimes_search::models::User::from(user_clone);
        user_search.update_document(&search_arc).await
    });
    let task_events = ctx
        .data_unchecked::<showtimes_events::SharedSHClickHouse>()
        .create_event_async(
            showtimes_events::m::EventKind::UserUpdated,
            showtimes_events::m::UserUpdatedEvent::new(user_info.id, user_before, user_after),
            if user.requester.kind == UserKind::Owner {
                None
            } else {
                Some(user.requester.id.to_string())
            },
        );

    execute_search_events(task_search, task_events).await?;

    Ok(APIKeyDataGQL::new(new_key, remapped_caps))
}

pub async fn mutate_users_authenticate(
    ctx: &async_graphql::Context<'_>,
    token: String,
    state: String,
) -> async_graphql::Result<UserSessionGQL> {
    let config = ctx.data_unchecked::<Arc<showtimes_shared::Config>>();
    let event_manager = ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();
    let sess_manager = ctx.data_unchecked::<SharedSessionManager>();
    let jwt_secret = ctx.data_unchecked::<showtimes_session::SharedSigner>();

    tracing::info!("Authenticating user with token: {}", &token);
    showtimes_session::verify_session(
        &state,
        jwt_secret,
        showtimes_session::ShowtimesAudience::DiscordAuth,
    )
    .map_err(|err| {
        let sel_error = match err {
            showtimes_session::SessionError::ValidationError(
                showtimes_session::SessionValidationError::TokenExpired(_, _),
            ) => GQLErrorCode::ExpiredToken,
            showtimes_session::SessionError::ValidationError(
                showtimes_session::SessionValidationError::InvalidAudience(_),
            ) => GQLErrorCode::UserInvalidAudience,
            showtimes_session::SessionError::InvalidSignature => GQLErrorCode::UserInvalidSignature,
            _ => GQLErrorCode::InvalidToken,
        };
        GQLError::new(err.to_string(), sel_error).extend(|e| {
            e.set("token", &token);
            e.set("state", &state);
        })
    })?;

    // Valid!
    let discord = ctx.data_unchecked::<Arc<DiscordClient>>();

    tracing::info!("Exchanging code {} for OAuth2 token...", &token);
    let exchanged = discord
        .exchange_code(&token, &config.discord.redirect_url)
        .await
        .map_err(|err| {
            GQLError::new(err.to_string(), GQLErrorCode::SessionExchangeError).extend(|f| {
                f.set("token", &token);
                f.set("state", &state);
            })
        })?;

    tracing::info!("Success, getting user for code {}", &token);
    let user_info = discord
        .get_user(&exchanged.access_token)
        .await
        .map_err(|err| {
            GQLError::new(err.to_string(), GQLErrorCode::SessionUserInfoError).extend(|f| {
                f.set("token", &token);
                f.set("state", &state);
            })
        })?;

    // Load handler and data loader
    let handler = showtimes_db::UserHandler::new(ctx.data_unchecked::<DatabaseShared>());
    let loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();

    tracing::info!("Checking if user exists for ID: {}", &user_info.id);
    let user = loader.load_one(DiscordIdLoad(user_info.id.clone())).await?;

    let expire_dur = jiff::SignedDuration::new(exchanged.expires_in as i64, 0);
    let auth_expire_at = jiff::Timestamp::now()
        .to_zoned(jiff::tz::TimeZone::UTC)
        .saturating_add(expire_dur);

    match user {
        Some(mut user) => {
            tracing::info!("User found, updating token for ID: {}", &user_info.id);
            let mut before_user = showtimes_events::m::UserUpdatedDataEvent::default();
            let mut after_user = showtimes_events::m::UserUpdatedDataEvent::default();
            before_user.set_discord_meta(&user.discord_meta);
            // Update the user token
            user.discord_meta.access_token = exchanged.access_token;
            if let Some(refresh_token) = &exchanged.refresh_token {
                user.discord_meta.refresh_token = refresh_token.to_string();
            } else {
                user.discord_meta.refresh_token = String::new();
            }
            user.discord_meta.expires_at = auth_expire_at.timestamp().as_second();

            if !user.registered {
                user.discord_meta.username = user_info.username.clone();
                before_user.set_name(&user.username);
                user.username = user_info.username.clone();
                after_user.set_name(&user.username);
                user.registered = true;
            }

            after_user.set_discord_meta(&user.discord_meta);

            handler.save(&mut user, None).await.extend_error(
                GQLErrorCode::UserUpdateError,
                |f| {
                    f.set("id", user.id.to_string());
                    f.set("token", &token);
                    f.set("state", &state);
                    f.set("is_new", false);
                },
            )?;

            let expiry_u64 = config.jwt.get_expiration();

            let (oauth_user, refresh_token) = showtimes_session::create_session(
                user.id,
                expiry_u64.try_into().map_err(|_| {
                    GQLError::new(
                        format!("Failed to convert {expiry_u64} into timestamp"),
                        GQLErrorCode::SessionCreateError,
                    )
                    .extend(|f| {
                        f.set("id", user.id.to_string());
                        f.set("token", &token);
                        f.set("state", &state);
                        f.set("is_new", false);
                        f.set("expiry", expiry_u64);
                    })
                })?,
                jwt_secret,
            )
            .extend_error(GQLErrorCode::SessionCreateError, |f| {
                f.set("id", user.id.to_string());
                f.set("token", &token);
                f.set("state", &state);
                f.set("is_new", false);
            })?;

            // Emit event
            event_manager
                .create_event(
                    showtimes_events::m::EventKind::UserUpdated,
                    showtimes_events::m::UserUpdatedEvent::new(user.id, before_user, after_user),
                    Some(user.id.to_string()),
                )
                .await
                .extend_error(GQLErrorCode::UserEventCreateError, |f| {
                    f.set("id", user.id.to_string());
                    f.set("token", &token);
                    f.set("state", &state);
                    f.set("is_new", false);
                    f.set(
                        "mode",
                        showtimes_events::m::EventKind::UserUpdated.to_name(),
                    );
                })?;

            let mut sess_mutex = sess_manager.lock().await;

            sess_mutex
                .set_session(oauth_user.get_token(), oauth_user.get_claims())
                .await
                .extend_error(GQLErrorCode::SessionStoreError, |f| {
                    f.set("id", user.id.to_string());
                    f.set("token", &token);
                    f.set("state", &state);
                    f.set("is_new", false);
                })?;
            sess_mutex
                .set_refresh_session(&refresh_token, oauth_user.get_token())
                .await
                .extend_error(GQLErrorCode::SessionRefreshStoreError, |f| {
                    f.set("id", user.id.to_string());
                    f.set("token", &token);
                    f.set("state", &state);
                    f.set("is_new", false);
                })?;
            drop(sess_mutex);

            Ok(UserSessionGQL::new(&user, oauth_user.get_token())
                .with_refresh_token(&refresh_token))
        }
        None => {
            tracing::info!(
                "User not found, creating new user for ID: {}",
                &user_info.id
            );
            // Create new user
            let discord_user = showtimes_db::m::DiscordUser {
                id: user_info.id,
                username: user_info.username.clone(),
                avatar: user_info.avatar,
                access_token: exchanged.access_token,
                refresh_token: exchanged.refresh_token.unwrap_or_default(),
                expires_at: auth_expire_at.timestamp().as_second(),
            };

            let mut user = showtimes_db::m::User::new(user_info.username, discord_user);
            handler.save(&mut user, None).await.extend_error(
                GQLErrorCode::UserCreateError,
                |f| {
                    f.set("id", user.id.to_string());
                    f.set("token", &token);
                    f.set("state", &state);
                    f.set("is_new", true);
                },
            )?;

            // Emit event
            event_manager
                .create_event(
                    showtimes_events::m::EventKind::UserCreated,
                    showtimes_events::m::UserCreatedEvent::from(&user),
                    Some(user.id.to_string()),
                )
                .await
                .extend_error(GQLErrorCode::UserEventCreateError, |f| {
                    f.set("id", user.id.to_string());
                    f.set("token", &token);
                    f.set("state", &state);
                    f.set("is_new", true);
                    f.set(
                        "mode",
                        showtimes_events::m::EventKind::UserCreated.to_name(),
                    );
                })?;

            let expiry_u64 = config.jwt.get_expiration();

            let (oauth_user, refresh_token) = showtimes_session::create_session(
                user.id,
                expiry_u64.try_into().map_err(|_| {
                    GQLError::new(
                        format!("Failed to convert {expiry_u64} into timestamp"),
                        GQLErrorCode::SessionCreateError,
                    )
                    .extend(|f| {
                        f.set("id", user.id.to_string());
                        f.set("token", &token);
                        f.set("state", &state);
                        f.set("is_new", false);
                        f.set("expiry", expiry_u64);
                    })
                })?,
                jwt_secret,
            )
            .extend_error(GQLErrorCode::SessionCreateError, |f| {
                f.set("id", user.id.to_string());
                f.set("token", &token);
                f.set("state", &state);
                f.set("is_new", true);
            })?;

            let mut sess_mutex = sess_manager.lock().await;

            sess_mutex
                .set_session(oauth_user.get_token(), oauth_user.get_claims())
                .await
                .extend_error(GQLErrorCode::SessionStoreError, |f| {
                    f.set("id", user.id.to_string());
                    f.set("token", &token);
                    f.set("state", &state);
                    f.set("is_new", true);
                })?;
            sess_mutex
                .set_refresh_session(&refresh_token, oauth_user.get_token())
                .await
                .extend_error(GQLErrorCode::SessionRefreshStoreError, |f| {
                    f.set("id", user.id.to_string());
                    f.set("token", &token);
                    f.set("state", &state);
                    f.set("is_new", true);
                })?;
            drop(sess_mutex);

            Ok(UserSessionGQL::new(&user, oauth_user.get_token())
                .with_refresh_token(&refresh_token))
        }
    }
}
