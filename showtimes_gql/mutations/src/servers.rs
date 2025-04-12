use std::sync::Arc;

use async_graphql::{InputObject, Upload, dataloader::DataLoader};
use showtimes_db::{
    DatabaseShared, ServerHandler,
    m::{ShowModelHandler, UserKind},
    mongodb::bson::doc,
};
use showtimes_fs::{FsFileKind, FsPool};
use showtimes_search::SearchClientShared;
use tokio::io::AsyncSeekExt;

use showtimes_gql_common::{
    DateTimeGQL, GQLErrorCode, GQLErrorExt, OkResponse, UlidGQL, UserKindGQL,
    data_loader::{ServerDataLoader, ServerPremiumLoader},
    errors::GQLError,
};
use showtimes_gql_models::servers::{ServerGQL, ServerPremiumGQL};

use crate::{
    IntegrationActionGQL, IntegrationInputGQL, IntegrationValidator, execute_search_events,
    is_string_set, is_vec_set,
};

/// The server input object for creating a new server
#[derive(InputObject)]
pub struct ServerCreateInputGQL {
    /// The server name
    #[graphql(validator(min_length = 5, max_length = 128))]
    name: String,
    /// The list of integration to add, update, or remove
    #[graphql(validator(
        custom = "IntegrationValidator::with_limit(vec![IntegrationActionGQL::Add])"
    ))]
    integrations: Option<Vec<IntegrationInputGQL>>,
    /// The server avatar
    avatar: Option<Upload>,
    /// Other owners of the server
    owners: Option<Vec<ServerOwnerInputGQL>>,
}

impl ServerCreateInputGQL {
    /// Dump the input into a new server
    fn dump_query(&self, f_mut: &mut async_graphql::ErrorExtensionValues) {
        f_mut.set("name", &self.name);
        f_mut.set("has_avatar", self.avatar.is_some());
        if let Some(owners) = &self.owners {
            f_mut.set(
                "owners",
                owners
                    .iter()
                    .map(|d| {
                        let mut f_new = async_graphql::indexmap::IndexMap::new();
                        d.dump_query(&mut f_new);
                        async_graphql::Value::Object(f_new)
                    })
                    .collect::<Vec<async_graphql::Value>>(),
            );
        }
        if let Some(integrations) = &self.integrations {
            f_mut.set(
                "integrations",
                integrations
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

/// The server input object on what to update
///
/// All fields are optional
#[derive(InputObject)]
pub struct ServerUpdateInputGQL {
    /// The server name
    #[graphql(validator(min_length = 3, max_length = 128))]
    name: Option<String>,
    /// The list of integration to add, update, or remove
    #[graphql(validator(custom = "IntegrationValidator::new()"))]
    integrations: Option<Vec<IntegrationInputGQL>>,
    /// The server avatar
    avatar: Option<Upload>,
}

impl ServerUpdateInputGQL {
    /// Check if any field is set
    fn is_any_set(&self) -> bool {
        is_string_set(&self.name) || is_vec_set(&self.integrations) || self.avatar.is_some()
    }

    fn dump_query(&self, f_mut: &mut async_graphql::ErrorExtensionValues) {
        if let Some(name) = &self.name {
            f_mut.set("name", name);
        }
        f_mut.set("has_avatar", self.avatar.is_some());
        if let Some(integrations) = &self.integrations {
            f_mut.set(
                "integrations",
                integrations
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

/// A server owner information input object
#[derive(InputObject)]
pub struct ServerOwnerInputGQL {
    /// The user ID
    id: UlidGQL,
    /// The user privilege
    kind: UserKindGQL,
    /// Additional information for this user
    extras: Option<Vec<String>>,
}

impl ServerOwnerInputGQL {
    /// Dump the input into a new server
    fn dump_query(
        &self,
        f_mut: &mut async_graphql::indexmap::IndexMap<async_graphql::Name, async_graphql::Value>,
    ) {
        f_mut.insert(async_graphql::Name::new("id"), self.id.to_string().into());
        f_mut.insert(async_graphql::Name::new("name"), self.kind.to_name().into());
        if let Some(extras) = &self.extras {
            f_mut.insert(async_graphql::Name::new("extras"), extras.to_vec().into());
        }
    }
}

pub async fn mutate_servers_create(
    ctx: &async_graphql::Context<'_>,
    input: ServerCreateInputGQL,
) -> async_graphql::Result<ServerGQL> {
    let db = ctx.data_unchecked::<DatabaseShared>();
    let meili = ctx.data_unchecked::<SearchClientShared>();
    let user = ctx.data_unchecked::<showtimes_db::m::User>();

    if user.kind == UserKind::Owner {
        // Fails, Owner cannot create server
        return GQLError::new(
            "This account cannot create a server",
            GQLErrorCode::UserSuperuserMode,
        )
        .extend(|e| {
            e.set("id", user.id.to_string());
        })
        .into();
    }

    let current_user = vec![showtimes_db::m::ServerUser::new(
        user.id,
        showtimes_db::m::UserPrivilege::Owner,
    )];

    let mut server = showtimes_db::m::Server::new(&input.name, current_user);

    if let Some(integrations) = &input.integrations {
        for integration in integrations {
            match integration.action {
                IntegrationActionGQL::Add => {
                    server.add_integration(integration.into());
                }
                _ => {
                    return GQLError::new(
                        "Only add action is allowed for new servers",
                        GQLErrorCode::InvalidRequest,
                    )
                    .extend(|e| {
                        e.set("id", integration.id.clone());
                        e.set("kind", integration.kind.to_string());
                        e.set("user_id", user.id.to_string());
                    })
                    .into();
                }
            }
        }
    }

    match input.avatar {
        Some(avatar_upload) => {
            let info_up = avatar_upload.value(ctx).map_err(|err| {
                GQLError::new(
                    format!("Failed to read upload image: {err}"),
                    GQLErrorCode::IOError,
                )
                .extend(|e| {
                    e.set("id", server.id.to_string());
                    e.set("where", "server");
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
                        e.set("id", server.id.to_string());
                        e.set("where", "server");
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
                        e.set("id", server.id.to_string());
                        e.set("where", "server");
                        e.set("original", format!("{err}"));
                        e.set("original_code", format!("{}", err.kind()));
                    })
                })?;

            let filename = format!("avatar.{}", format.as_extension());

            let storages = ctx.data_unchecked::<Arc<FsPool>>();
            storages
                .file_stream_upload(
                    server.id,
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
                        e.set("id", server.id.to_string());
                        e.set("where", "server");
                        e.set("original", format!("{err}"));
                    })
                })?;

            let image_meta = showtimes_db::m::ImageMetadata::new(
                showtimes_fs::FsFileKind::Images.to_name(),
                server.id,
                &filename,
                format.as_extension(),
                None::<String>,
            );

            server.avatar = Some(image_meta);
        }
        None => {
            server.avatar = Some(showtimes_db::m::ImageMetadata::new(
                FsFileKind::Invalids.to_name(),
                "server",
                "default.png",
                "png",
                None::<String>,
            ));
        }
    }

    // Commit to database
    let srv_handler = ServerHandler::new(db);
    srv_handler
        .save_direct(&mut server, None)
        .await
        .extend_error(GQLErrorCode::ServerCreateError, |f_mut| {
            f_mut.set("id", server.id.to_string());
            f_mut.set("user", user.id.to_string());
            input.dump_query(f_mut);
        })?;

    // Commit to search engine
    let server_clone = server.clone();
    let meili_clone = meili.clone();
    let task_search = tokio::task::spawn(async move {
        let srv_search = showtimes_search::models::Server::from(server_clone);
        srv_search.update_document(&meili_clone).await
    });
    // Commit to events
    let task_events = ctx
        .data_unchecked::<showtimes_events::SharedSHClickHouse>()
        .create_event_async(
            showtimes_events::m::EventKind::ServerCreated,
            showtimes_events::m::ServerCreatedEvent::from(&server),
            if user.kind == UserKind::Owner {
                None
            } else {
                Some(user.id.to_string())
            },
        );

    execute_search_events(task_search, task_events).await?;

    Ok(server.into())
}

async fn get_and_check_server(
    ctx: &async_graphql::Context<'_>,
    id: showtimes_shared::ulid::Ulid,
    user: &showtimes_db::m::User,
    min_privilege: showtimes_db::m::UserPrivilege,
) -> async_graphql::Result<showtimes_db::m::Server> {
    let loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();
    let server = loader.load_one(id).await?.ok_or_else(|| {
        GQLError::new("Server not found", GQLErrorCode::ServerNotFound)
            .extend(|e| e.set("id", id.to_string()))
    })?;

    let user_owner = server.owners.iter().find(|o| o.id == user.id);
    let user_owner = match (user.kind, user_owner) {
        (UserKind::User, Some(user_owner)) => user_owner.clone(),
        (UserKind::User, None) => {
            return GQLError::new(
                "User does not have permission to update the server",
                GQLErrorCode::UserInsufficientPrivilege,
            )
            .extend(|e| {
                e.set("id", id.to_string());
                e.set("user", user.id.to_string());
                e.set("is_in_server", false);
            })
            .into();
        }
        // Admin and Owner has "Owner" privilege
        (_, _) => showtimes_db::m::ServerUser::new(user.id, showtimes_db::m::UserPrivilege::Owner),
    };

    // Anything below min_privilege is disallowed
    if user_owner.privilege < min_privilege {
        return GQLError::new(
            "User does not have permission to update the server",
            GQLErrorCode::UserInsufficientPrivilege,
        )
        .extend(|e| {
            e.set("id", id.to_string());
            e.set("user", user.id.to_string());
            e.set("current", user_owner.privilege.to_string());
            e.set("minimum", min_privilege.to_string());
            e.set("is_in_server", false);
        })
        .into();
    }

    Ok(server)
}

pub async fn mutate_servers_update(
    ctx: &async_graphql::Context<'_>,
    id: UlidGQL,
    input: ServerUpdateInputGQL,
) -> async_graphql::Result<ServerGQL> {
    if !input.is_any_set() {
        return GQLError::new("No fields to update", GQLErrorCode::MissingModification).into();
    }

    let db = ctx.data_unchecked::<DatabaseShared>();
    let storages = ctx.data_unchecked::<Arc<FsPool>>();
    let meili = ctx.data_unchecked::<SearchClientShared>();
    let user = ctx.data_unchecked::<showtimes_db::m::User>();

    // Do update
    let server =
        get_and_check_server(ctx, *id, user, showtimes_db::m::UserPrivilege::Admin).await?;
    let mut server_mut = server.clone();

    let mut server_before = showtimes_events::m::ServerUpdatedDataEvent::default();
    let mut server_after = showtimes_events::m::ServerUpdatedDataEvent::default();

    if let Some(name) = &input.name {
        server_before.set_name(&server_mut.name);
        server_mut.name = name.to_string();
        server_after.set_name(&server_mut.name);
    }

    server_before.set_integrations(&server_mut.integrations);

    let mut any_integrations_changes = false;
    for (idx, integration) in input
        .integrations
        .clone()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        match (integration.action, integration.original_id.clone()) {
            (IntegrationActionGQL::Add, _) => {
                // Check if the integration already exists
                let same_integration = server_mut
                    .integrations
                    .iter()
                    .find(|i| i.id() == integration.id);

                if same_integration.is_none() {
                    server_mut.add_integration(integration.into());
                    any_integrations_changes = true;
                }
            }
            (IntegrationActionGQL::Update, Some(original_id)) => {
                // Get olf integration
                let old_integration = server
                    .integrations
                    .iter()
                    .find(|i| i.id() == original_id)
                    .ok_or_else(|| {
                        GQLError::new("Integration not found", GQLErrorCode::IntegrationNotFound)
                            .extend(|e| {
                                e.set("id", &original_id);
                                e.set("server", server_mut.id.to_string());
                                e.set("action", "update");
                            })
                    })?;

                // Update the integration
                let new_integration = integration.into();
                server_mut.remove_integration(old_integration);
                server_mut.add_integration(new_integration);
                any_integrations_changes = true;
            }
            (IntegrationActionGQL::Update, None) => {
                return GQLError::new(
                    "Integration original ID is required for update",
                    GQLErrorCode::IntegrationMissingOriginal,
                )
                .extend(|e| {
                    e.set("id", integration.id.to_string());
                    e.set("kind", integration.kind.to_string());
                    e.set("server", server_mut.id.to_string());
                    e.set("action", "update");
                    e.set("index", idx);
                })
                .into();
            }
            (IntegrationActionGQL::Remove, _) => {
                // Check if the integration exists
                let integration: showtimes_db::m::IntegrationId = integration.into();
                server_mut.remove_integration(&integration);
                any_integrations_changes = true;
            }
        }
    }

    if any_integrations_changes {
        server_after.set_integrations(&server_mut.integrations);
    } else {
        server_before.clear_integrations();
    }

    if let Some(avatar_upload) = input.avatar {
        let info_up = avatar_upload.value(ctx).map_err(|err| {
            GQLError::new(
                format!("Failed to read upload image: {err}"),
                GQLErrorCode::IOError,
            )
            .extend(|e| {
                e.set("id", server_mut.id.to_string());
                e.set("where", "server");
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
                    e.set("id", server_mut.id.to_string());
                    e.set("where", "server");
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
                    e.set("id", server_mut.id.to_string());
                    e.set("where", "server");
                    e.set("original", format!("{err}"));
                    e.set("original_code", format!("{}", err.kind()));
                })
            })?;

        let filename = format!("avatar.{}", format.as_extension());

        storages
            .file_stream_upload(
                server_mut.id,
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
                    e.set("id", server_mut.id.to_string());
                    e.set("where", "server");
                    e.set("original", format!("{err}"));
                })
            })?;

        let image_meta = showtimes_db::m::ImageMetadata::new(
            showtimes_fs::FsFileKind::Images.to_name(),
            server_mut.id,
            &filename,
            format.as_extension(),
            None::<String>,
        );

        if let Some(avatar) = &server_mut.avatar {
            server_before.set_avatar(avatar);
        }
        server_after.set_avatar(&image_meta);
        server_mut.avatar = Some(image_meta);
    }

    // Update the user
    let srv_handler = ServerHandler::new(db);
    srv_handler.save(&mut server_mut, None).await.extend_error(
        GQLErrorCode::ServerUpdateError,
        |f_mut| {
            f_mut.set("id", server_mut.id.to_string());
            f_mut.set("actor", user.id.to_string());
            input.dump_query(f_mut);
        },
    )?;

    // Update index
    let server_clone = server_mut.clone();
    let meili_clone = meili.clone();
    let task_search = tokio::task::spawn(async move {
        let srv_search = showtimes_search::models::Server::from(server_clone);
        srv_search.update_document(&meili_clone).await
    });
    // Commit to events
    let task_events = ctx
        .data_unchecked::<showtimes_events::SharedSHClickHouse>()
        .create_event_async(
            showtimes_events::m::EventKind::ServerUpdated,
            showtimes_events::m::ServerUpdatedEvent::new(
                server_mut.id,
                server_before,
                server_after,
            ),
            if user.kind == UserKind::Owner {
                None
            } else {
                Some(user.id.to_string())
            },
        );

    execute_search_events(task_search, task_events).await?;

    let srv_gql: ServerGQL = server_mut.into();

    Ok(srv_gql.with_current_user(user.id))
}

pub async fn mutate_servers_delete(
    ctx: &async_graphql::Context<'_>,
    id: UlidGQL,
) -> async_graphql::Result<OkResponse> {
    let db = ctx.data_unchecked::<DatabaseShared>();
    let storages = ctx.data_unchecked::<Arc<FsPool>>();
    let meili = ctx.data_unchecked::<SearchClientShared>();
    let user = ctx.data_unchecked::<showtimes_db::m::User>();

    // Get server info
    let server =
        get_and_check_server(ctx, *id, user, showtimes_db::m::UserPrivilege::Owner).await?;

    // Unlink Collab sync and invite
    let collab_handler = showtimes_db::CollaborationSyncHandler::new(db);
    let collab_info = collab_handler
        .find_all_by(doc! {
            "projects.server": server.id.to_string()
        })
        .await
        .extend_error(GQLErrorCode::ServerSyncRequestFails, |f_mut| {
            f_mut.set("id", id.to_string());
            f_mut.set("server_id", server.id.to_string());
        })?;

    let mut collab_deleted: Vec<String> = vec![];
    let mut collab_deleted_events: Vec<showtimes_events::m::CollabDeletedEvent> = vec![];
    let mut collab_updated: Vec<showtimes_db::m::ServerCollaborationSync> = vec![];
    let mut collab_updated_events: Vec<showtimes_events::m::CollabDeletedEvent> = vec![];
    for collab in collab_info {
        let mut collab_mut = collab.clone();
        let srv_collab_data = collab_mut.get_and_remove_server(server.id);
        if let Some(srv_collab_data) = srv_collab_data {
            // If only 1 or zero, delete this link
            if collab_mut.length() < 2 {
                // Delete from DB
                collab_handler.delete(&collab).await.extend_error(
                    GQLErrorCode::ServerSyncDeleteError,
                    |f| {
                        f.set("id", collab.id.to_string());
                        f.set("server_target", server.id.to_string());
                    },
                )?;

                // Delete from search engine
                collab_deleted_events.push(showtimes_events::m::CollabDeletedEvent::new(
                    collab.id,
                    &srv_collab_data,
                    true,
                ));

                collab_deleted.push(collab.id.to_string());
            } else {
                collab_handler
                    .save(&mut collab_mut, None)
                    .await
                    .extend_error(GQLErrorCode::ServerSyncUpdateError, |f| {
                        f.set("id", collab.id.to_string());
                        f.set("server_target", server.id.to_string());
                    })?;

                collab_updated_events.push(showtimes_events::m::CollabDeletedEvent::new(
                    collab.id,
                    &srv_collab_data,
                    false,
                ));
                collab_updated.push(collab_mut);
            }
        }
    }

    if !collab_deleted.is_empty() && !collab_deleted_events.is_empty() {
        let index_collab = meili.index(showtimes_search::models::ServerCollabSync::index_name());

        // Search adjustment
        let meili_clone = meili.clone();
        let task_search = tokio::task::spawn(async move {
            match index_collab.delete_documents(&collab_deleted).await {
                Ok(task_del) => {
                    match task_del.wait_for_completion(&meili_clone, None, None).await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    }
                }
                Err(e) => Err(e),
            }
        });

        // Commit to events
        let task_events = ctx
            .data_unchecked::<showtimes_events::SharedSHClickHouse>()
            .create_event_many_async(
                showtimes_events::m::EventKind::CollaborationDeleted,
                collab_deleted_events,
                if user.kind == UserKind::Owner {
                    None
                } else {
                    Some(user.id.to_string())
                },
            );

        execute_search_events(task_search, task_events).await?;
    }

    if !collab_updated.is_empty() && !collab_updated_events.is_empty() {
        let index_collab = meili.index(showtimes_search::models::ServerCollabSync::index_name());

        // Search adjustment
        let meili_clone = meili.clone();
        let task_search = tokio::task::spawn(async move {
            match index_collab
                .add_or_update(
                    &collab_updated,
                    Some(showtimes_search::models::ServerCollabSync::index_name()),
                )
                .await
            {
                Ok(task_del) => {
                    match task_del.wait_for_completion(&meili_clone, None, None).await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    }
                }
                Err(e) => Err(e),
            }
        });

        // Commit to events
        let task_events = ctx
            .data_unchecked::<showtimes_events::SharedSHClickHouse>()
            .create_event_many_async(
                showtimes_events::m::EventKind::CollaborationDeleted,
                collab_updated_events,
                if user.kind == UserKind::Owner {
                    None
                } else {
                    Some(user.id.to_string())
                },
            );

        execute_search_events(task_search, task_events).await?;
    }

    let collab_invite_handler = showtimes_db::CollaborationInviteHandler::new(db);
    let collab_invite_info = collab_invite_handler
        .find_all_by(doc! {
            "$or": [
                {
                    "source.server": server.id.to_string()
                },
                {
                    "target.server": server.id.to_string()
                }
            ]
        })
        .await
        .extend_error(GQLErrorCode::ServerInviteRequestFails, |f| {
            f.set("server_id", server.id.to_string());
        })?;

    let all_invite_ids = collab_invite_info
        .iter()
        .map(|c| c.id.to_string())
        .collect::<Vec<String>>();

    if !all_invite_ids.is_empty() {
        // Delete from DB
        collab_invite_handler
            .delete_by(doc! {
                "id": {
                    "$in": all_invite_ids.clone()
                }
            })
            .await
            .extend_error(GQLErrorCode::ServerInviteDeleteError, |f| {
                f.set("invite_ids", all_invite_ids.clone());
                f.set("server_id", server.id.to_string());
            })?;

        // Delete from search engine
        let index_invite = meili.index(showtimes_search::models::ServerCollabInvite::index_name());

        let meili_clone = meili.clone();
        let task_search = tokio::task::spawn(async move {
            match index_invite.delete_documents(&all_invite_ids).await {
                Ok(task_del) => {
                    match task_del.wait_for_completion(&meili_clone, None, None).await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    }
                }
                Err(e) => Err(e),
            }
        });

        // Create events for retracted
        let retracted_events: Vec<showtimes_events::m::CollabRetractedEvent> = collab_invite_info
            .iter()
            .map(|collab| showtimes_events::m::CollabRetractedEvent::new(collab.id))
            .collect();

        // Create task events
        let task_events = ctx
            .data_unchecked::<showtimes_events::SharedSHClickHouse>()
            .create_event_many_async(
                showtimes_events::m::EventKind::CollaborationRetracted,
                retracted_events,
                if user.kind == UserKind::Owner {
                    None
                } else {
                    Some(user.id.to_string())
                },
            );

        execute_search_events(task_search, task_events).await?;
    }

    // Delete projects
    let project_handler = showtimes_db::ProjectHandler::new(db);
    let project_info = project_handler
        .find_all_by(doc! {
            "creator": server.id.to_string()
        })
        .await
        .extend_error(GQLErrorCode::ProjectRequestFails, |f| {
            f.set("creator", server.id.to_string());
        })?;

    let all_project_ids = project_info
        .iter()
        .map(|p| p.id.to_string())
        .collect::<Vec<String>>();

    if !all_project_ids.is_empty() {
        // Delete from DB
        project_handler
            .delete_by(doc! {
                "id": {
                    "$in": all_project_ids.clone()
                }
            })
            .await
            .extend_error(GQLErrorCode::ProjectDeleteError, |f| {
                f.set("project_ids", all_project_ids.clone());
                f.set("server_id", server.id.to_string());
            })?;

        // Delete from search engine
        let index_project = meili.index(showtimes_search::models::Project::index_name());

        let meili_clone = meili.clone();
        let task_search = tokio::task::spawn(async move {
            match index_project.delete_documents(&all_project_ids).await {
                Ok(task_del) => {
                    match task_del.wait_for_completion(&meili_clone, None, None).await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    }
                }
                Err(e) => Err(e),
            }
        });

        // Create events for deleted
        let deleted_events: Vec<showtimes_events::m::ProjectDeletedEvent> = project_info
            .iter()
            .map(|project| showtimes_events::m::ProjectDeletedEvent::new(project.id))
            .collect();

        // Create task events
        let task_events = ctx
            .data_unchecked::<showtimes_events::SharedSHClickHouse>()
            .create_event_many_async(
                showtimes_events::m::EventKind::ProjectDeleted,
                deleted_events,
                if user.kind == UserKind::Owner {
                    None
                } else {
                    Some(user.id.to_string())
                },
            );

        execute_search_events(task_search, task_events).await?;
    }

    // Delete RSS feeds
    let rss_handler = showtimes_db::RSSFeedHandler::new(db);
    rss_handler
        .get_collection()
        .delete_many(doc! {
            "creator": server.id.to_string()
        })
        .await
        .extend_error(GQLErrorCode::RSSFeedDeleteError, |f| {
            f.set("creator", server.id.to_string());
        })?;

    // Delete premium related
    let premium_handler = showtimes_db::ServerPremiumHandler::new(db);
    premium_handler
        .delete_by(doc! {
            "target": server.id.to_string()
        })
        .await
        .extend_error(GQLErrorCode::ServerPremiumDeleteError, |f| {
            f.set("server_id", server.id.to_string());
        })?;

    // Delete assets
    storages
        .directory_delete(server.id, None, Some(showtimes_fs::FsFileKind::Images))
        .await
        .extend_error(GQLErrorCode::ImageBulkDeleteError, |f| {
            f.set("id", server.id.to_string());
            f.set("actor", user.id.to_string());
        })?;

    // Delete from DB
    let srv_handler = ServerHandler::new(db);
    srv_handler
        .delete(&server)
        .await
        .extend_error(GQLErrorCode::ServerDeleteError, |f| {
            f.set("id", server.id.to_string());
            f.set("actor", user.id.to_string());
        })?;

    // Delete from search engine
    let srv_clone = server.clone();
    let meili_clone = meili.clone();
    let task_search = tokio::task::spawn(async move {
        let srv_search = showtimes_search::models::Server::from(srv_clone);
        srv_search.delete_document(&meili_clone).await
    });
    // Commit to events
    let task_events = ctx
        .data_unchecked::<showtimes_events::SharedSHClickHouse>()
        .create_event_async(
            showtimes_events::m::EventKind::ServerDeleted,
            showtimes_events::m::ServerDeletedEvent::new(server.id),
            if user.kind == UserKind::Owner {
                None
            } else {
                Some(user.id.to_string())
            },
        );

    execute_search_events(task_search, task_events).await?;

    Ok(OkResponse::ok("Server deleted"))
}

pub async fn mutate_servers_premium_create(
    ctx: &async_graphql::Context<'_>,
    id: UlidGQL,
    ends_at: DateTimeGQL,
) -> async_graphql::Result<ServerPremiumGQL> {
    let user = ctx.data_unchecked::<showtimes_db::m::User>();

    if user.kind < UserKind::Admin {
        // Fails, needs admin perms minimum perms
        return GQLError::new(
            "This account cannot create a new premium status for a server",
            GQLErrorCode::UserInsufficientPrivilege,
        )
        .extend(|e| {
            e.set("id", id.to_string());
            e.set("user_id", user.id.to_string());
        })
        .into();
    }

    // Check if server exist
    let srv_loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();
    let server = srv_loader.load_one(*id).await?.ok_or_else(|| {
        GQLError::new("Server not found", GQLErrorCode::ServerNotFound)
            .extend(|e| e.set("id", id.to_string()))
    })?;

    let loader = ctx.data_unchecked::<DataLoader<ServerPremiumLoader>>();
    let handler = loader.loader().get_inner();

    let current_time = jiff::Timestamp::now();
    let current_time_bson =
        showtimes_db::mongodb::bson::DateTime::from_millis(current_time.as_millisecond());

    // Check if ends_at is in the past
    if current_time > *ends_at {
        return GQLError::new(
            "Ends at time is in the past",
            GQLErrorCode::ServerPremiumInvalidEndTime,
        )
        .extend(|e| {
            e.set("id", server.id.to_string());
            e.set("ends_at", ends_at.to_string());
            e.set("current_time", current_time.to_string());
        })
        .into();
    }

    // Find for an active premium
    let active_premium = handler
        .find_by(doc! {
            "target": server.id.to_string(),
            // Each premium ends at specific time, ensure we only get the one that is active right now.
            "ends_at": { "$gte": current_time_bson }
        })
        .await
        .extend_error(GQLErrorCode::ServerPremiumRequestFails, |e| {
            e.set("id", server.id.to_string());
        })?;

    let mut premium = match active_premium {
        Some(premium) => {
            // Check if ends_at is less than the current active premium
            // if yes, use extend_by method by the duration between ends_at and current time
            if *ends_at < premium.ends_at {
                let ends_at_dur = *ends_at - current_time;

                if ends_at_dur.is_negative() {
                    // negative?!
                    return GQLError::new(
                        "Ends at time is in the past",
                        GQLErrorCode::ServerPremiumInvalidEndTime,
                    )
                    .extend(|e| {
                        e.set("id", server.id.to_string());
                        e.set("ends_at", ends_at.to_string());
                        e.set("current_time", premium.ends_at.to_string());
                    })
                    .into();
                }

                // extend by the duration
                premium.extend_by(ends_at_dur)
            } else {
                // extend until ends_at
                premium.extend_at(*ends_at)
            }
        }
        None => {
            // Create a new premium
            showtimes_db::m::ServerPremium::new(server.id, *ends_at)
        }
    };

    premium.updated();

    // Commit to DB
    handler.save_direct(&mut premium, None).await.map_err(|e| {
        GQLError::new(e.to_string(), GQLErrorCode::ServerPremiumCreateError).extend(|f| {
            f.set("id", server.id.to_string());
            f.set("ends_at", ends_at.to_string());
        })
    })?;

    Ok(ServerPremiumGQL::from(premium).with_current_user(user.id))
}

pub async fn mutate_servers_premium_delete(
    ctx: &async_graphql::Context<'_>,
    id: UlidGQL,
) -> async_graphql::Result<OkResponse> {
    let user = ctx.data_unchecked::<showtimes_db::m::User>();
    let db = ctx.data_unchecked::<DatabaseShared>();

    if user.kind < UserKind::Admin {
        return GQLError::new(
            "This account cannot delete a premium status for a server",
            GQLErrorCode::UserInsufficientPrivilege,
        )
        .extend(|e| {
            e.set("id", id.to_string());
            e.set("user_id", user.id.to_string());
        })
        .into();
    }

    // Load premium information
    let loader = ctx.data_unchecked::<DataLoader<ServerPremiumLoader>>();

    let premium = loader.load_one(*id).await?.ok_or_else(|| {
        GQLError::new(
            "Premium status not found",
            GQLErrorCode::ServerPremiumNotFound,
        )
        .extend(|e| e.set("id", id.to_string()))
    })?;

    // Get server information
    let srv_loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();
    let server = srv_loader.load_one(premium.target).await?.ok_or_else(|| {
        GQLError::new("Server not found", GQLErrorCode::ServerNotFound)
            .extend(|e| e.set("id", premium.target.to_string()))
    })?;

    // Handle RSS feeds that may need to be disabled
    let rss_handler = showtimes_db::RSSFeedHandler::new(db);
    let rss_feeds = rss_handler
        .find_all_by(doc! {
            "creator": server.id.to_string()
        })
        .await
        .extend_error(GQLErrorCode::RSSFeedRequestFails, |e| {
            e.set("server_id", server.id.to_string());
        })?;

    // Get server config for max RSS feeds without premium
    let config = ctx.data_unchecked::<Arc<showtimes_shared::Config>>();
    let max_standard_feeds = config.rss.standard_limit.unwrap_or(2);

    // If there are more RSS feeds than allowed without premium, disable newer ones
    let to_be_disabled: Vec<String> = if rss_feeds.len() > max_standard_feeds as usize {
        // Sort by creation date, ascending (oldest first)
        let mut sorted_feeds = rss_feeds.clone();
        sorted_feeds.sort_by(|a, b| a.created.cmp(&b.created));

        // Skip the max allowed feeds (keep oldest ones) and disable the rest
        sorted_feeds
            .iter()
            .skip(max_standard_feeds as usize)
            .map(|feed| feed.id.to_string())
            .collect()
    } else {
        vec![]
    };

    if !to_be_disabled.is_empty() {
        // Disable the RSS feeds
        rss_handler
            .get_collection()
            .update_many(
                doc! {
                    "id": {
                        "$in": to_be_disabled
                    }
                },
                doc! {
                    "$set": {
                        "enabled": false
                    }
                },
            )
            .await
            .extend_error(GQLErrorCode::RSSFeedUpdateError, |e| {
                e.set("server_id", server.id.to_string());
            })?;
    }

    // Delete the premium status
    loader
        .loader()
        .get_inner()
        .delete(&premium)
        .await
        .extend_error(GQLErrorCode::ServerPremiumDeleteError, |e| {
            e.set("id", premium.id.to_string());
            e.set("server_id", server.id.to_string());
        })?;

    Ok(OkResponse::ok("Server premium deleted"))
}
