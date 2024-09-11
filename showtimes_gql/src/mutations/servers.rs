use std::sync::Arc;

use async_graphql::{dataloader::DataLoader, Error, ErrorExtensions, InputObject, Upload};
use showtimes_db::{m::UserKind, mongodb::bson::doc, DatabaseShared, ServerHandler};
use showtimes_fs::{FsFileKind, FsPool};
use showtimes_search::SearchClientShared;
use tokio::io::AsyncSeekExt;

use crate::{
    data_loader::ServerDataLoader,
    models::{
        prelude::{OkResponse, UlidGQL},
        servers::ServerGQL,
        users::UserKindGQL,
    },
};

use super::{IntegrationActionGQL, IntegrationInputGQL, IntegrationValidator};

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

/// The server input object on what to update
///
/// All fields are optional
#[derive(InputObject)]
pub struct ServerUpdateInputGQL {
    /// The server name
    #[graphql(validator(min_length = 5, max_length = 128))]
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
        self.name.is_some() || self.integrations.is_some() || self.avatar.is_some()
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

pub async fn mutate_servers_create(
    ctx: &async_graphql::Context<'_>,
    user: showtimes_db::m::User,
    input: ServerCreateInputGQL,
) -> async_graphql::Result<ServerGQL> {
    let db = ctx.data_unchecked::<DatabaseShared>();
    let meili = ctx.data_unchecked::<SearchClientShared>();

    let current_user = vec![showtimes_db::m::ServerUser::new(
        user.id,
        showtimes_db::m::UserPrivilege::Owner,
    )];

    let mut server = showtimes_db::m::Server::new(&input.name, current_user);

    if let Some(integrations) = input.integrations {
        for integration in integrations {
            match integration.action {
                IntegrationActionGQL::Add => {
                    server.add_integration(integration.into());
                }
                _ => {
                    return Err(Error::new("Only add action is allowed for new servers")
                        .extend_with(|_, e| {
                            e.set("id", integration.id.clone());
                            e.set("kind", integration.kind.to_string());
                        }));
                }
            }
        }
    }

    match input.avatar {
        Some(avatar_upload) => {
            let info_up = avatar_upload.value(ctx)?;
            let mut file_target = tokio::fs::File::from_std(info_up.content);

            // Get format
            let format = crate::image::detect_upload_data(&mut file_target).await?;
            // Seek back to the start of the file
            file_target.seek(std::io::SeekFrom::Start(0)).await?;

            let filename = format!("avatar.{}", format.as_extension());

            let storages = ctx.data_unchecked::<Arc<FsPool>>();
            storages
                .file_stream_upload(
                    server.id,
                    &filename,
                    &mut file_target,
                    None,
                    Some(showtimes_fs::FsFileKind::Images),
                )
                .await?;

            let image_meta = showtimes_db::m::ImageMetadata::new(
                showtimes_fs::FsFileKind::Images.as_path_name(),
                server.id,
                &filename,
                format.as_extension(),
                None::<String>,
            );

            server.avatar = Some(image_meta);
        }
        None => {
            server.avatar = Some(showtimes_db::m::ImageMetadata::new(
                FsFileKind::Invalids.as_path_name(),
                "server",
                "default.png",
                "png",
                None::<String>,
            ));
        }
    }

    // Commit to database
    let srv_handler = ServerHandler::new(db);
    srv_handler.save_direct(&mut server, None).await?;

    // Commit to search engine
    let srv_search = showtimes_search::models::Server::from(server.clone());
    srv_search.update_document(meili).await?;

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
        Error::new("Server not found").extend_with(|_, e| e.set("id", id.to_string()))
    })?;

    let user_owner = server.owners.iter().find(|o| o.id == user.id);
    let user_owner = match (user.kind, user_owner) {
        (UserKind::User, Some(user_owner)) => user_owner.clone(),
        (UserKind::User, None) => {
            return Err(
                Error::new("User does not have permission to update the server").extend_with(
                    |_, e| {
                        e.set("id", id.to_string());
                        e.set("user", user.id.to_string());
                    },
                ),
            );
        }
        // Admin and Owner has "Owner" privilege
        (_, _) => showtimes_db::m::ServerUser::new(user.id, showtimes_db::m::UserPrivilege::Owner),
    };

    // Anything below min_privilege is disallowed
    if user_owner.privilege < min_privilege {
        return Err(
            Error::new("User does not have permission to update the server").extend_with(|_, e| {
                e.set("id", id.to_string());
                e.set("user", user.id.to_string());
                e.set("privilege", user_owner.privilege.to_string());
                e.set("min_privilege", min_privilege.to_string());
            }),
        );
    }

    Ok(server)
}

pub async fn mutate_servers_update(
    ctx: &async_graphql::Context<'_>,
    id: UlidGQL,
    user: showtimes_db::m::User,
    input: ServerUpdateInputGQL,
) -> async_graphql::Result<ServerGQL> {
    if !input.is_any_set() {
        return Err(Error::new("No fields to update"));
    }

    let db = ctx.data_unchecked::<DatabaseShared>();
    let storages = ctx.data_unchecked::<Arc<FsPool>>();
    let meili = ctx.data_unchecked::<SearchClientShared>();

    // Do update
    let server =
        get_and_check_server(ctx, *id, &user, showtimes_db::m::UserPrivilege::Admin).await?;
    let mut server_mut = server.clone();
    if let Some(name) = input.name {
        server_mut.name = name;
    }

    for (idx, integration) in input.integrations.unwrap_or_default().iter().enumerate() {
        match (integration.action, integration.original_id.clone()) {
            (IntegrationActionGQL::Add, _) => {
                // Check if the integration already exists
                let same_integration = server_mut
                    .integrations
                    .iter()
                    .find(|i| i.id() == integration.id);

                if same_integration.is_none() {
                    server_mut.add_integration(integration.into());
                }
            }
            (IntegrationActionGQL::Update, Some(original_id)) => {
                // Get olf integration
                let old_integration = server
                    .integrations
                    .iter()
                    .find(|i| i.id() == original_id)
                    .ok_or_else(|| {
                        Error::new("Integration not found").extend_with(|_, e| {
                            e.set("id", original_id.to_string());
                            e.set("server", server_mut.id.to_string());
                        })
                    })?;

                // Update the integration
                let new_integration = integration.into();
                server_mut.remove_integration(old_integration);
                server_mut.add_integration(new_integration);
            }
            (IntegrationActionGQL::Update, None) => {
                return Err(
                    Error::new("Original ID is required for update").extend_with(|_, e| {
                        e.set("id", integration.id.to_string());
                        e.set("kind", integration.kind.to_string());
                        e.set("index", idx);
                        e.set("server", server_mut.id.to_string());
                    }),
                );
            }
            (IntegrationActionGQL::Remove, _) => {
                // Check if the integration exists
                let integration: showtimes_db::m::IntegrationId = integration.into();
                server_mut.remove_integration(&integration);
            }
        }
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
                server_mut.id,
                &filename,
                &mut file_target,
                None,
                Some(showtimes_fs::FsFileKind::Images),
            )
            .await?;

        let image_meta = showtimes_db::m::ImageMetadata::new(
            showtimes_fs::FsFileKind::Images.as_path_name(),
            server_mut.id,
            &filename,
            format.as_extension(),
            None::<String>,
        );

        server_mut.avatar = Some(image_meta);
    }

    // Update the user
    let srv_handler = ServerHandler::new(db);
    srv_handler.save(&mut server_mut, None).await?;

    // Update index
    let srv_search = showtimes_search::models::Server::from(server_mut.clone());
    srv_search.update_document(meili).await?;

    let srv_gql: ServerGQL = server_mut.into();

    Ok(srv_gql.with_current_user(user.id))
}

pub async fn mutate_servers_delete(
    ctx: &async_graphql::Context<'_>,
    user: showtimes_db::m::User,
    id: UlidGQL,
) -> async_graphql::Result<OkResponse> {
    let db = ctx.data_unchecked::<DatabaseShared>();
    let storages = ctx.data_unchecked::<Arc<FsPool>>();
    let meili = ctx.data_unchecked::<SearchClientShared>();

    // Get server info
    let server =
        get_and_check_server(ctx, *id, &user, showtimes_db::m::UserPrivilege::Owner).await?;

    // Unlink Collab sync and invite
    let collab_handler = showtimes_db::CollaborationSyncHandler::new(db);
    let collab_info = collab_handler
        .find_all_by(doc! {
            "projects.server": server.id.to_string()
        })
        .await?;

    let mut collab_deleted: Vec<String> = vec![];
    let mut collab_updated: Vec<showtimes_db::m::ServerCollaborationSync> = vec![];
    for collab in collab_info {
        let mut collab_mut = collab.clone();
        collab_mut.projects.retain(|p| p.server != server.id);

        // If only 1 or zero, delete this link
        if collab_mut.projects.len() < 2 {
            // Delete from DB
            collab_handler.delete(&collab).await?;

            // Delete from search engine
            collab_deleted.push(collab.id.to_string());
        } else {
            collab_handler.save(&mut collab_mut, None).await?;
            collab_updated.push(collab_mut);
        }
    }

    if !collab_deleted.is_empty() {
        let index_collab = meili.index(showtimes_search::models::ServerCollabSync::index_name());
        let task_del = index_collab.delete_documents(&collab_deleted).await?;
        task_del.wait_for_completion(meili, None, None).await?;
    }

    if !collab_updated.is_empty() {
        let index_collab = meili.index(showtimes_search::models::ServerCollabSync::index_name());
        let task_update = index_collab
            .add_or_update(
                &collab_updated,
                Some(showtimes_search::models::ServerCollabSync::index_name()),
            )
            .await?;
        task_update.wait_for_completion(meili, None, None).await?;
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
        .await?;

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
            .await?;

        // Delete from search engine
        let index_invite = meili.index(showtimes_search::models::ServerCollabInvite::index_name());

        let task_del = index_invite.delete_documents(&all_invite_ids).await?;
        task_del.wait_for_completion(meili, None, None).await?;
    }

    // Delete projects
    let project_handler = showtimes_db::ProjectHandler::new(db);
    let project_info = project_handler
        .find_all_by(doc! {
            "creator": server.id.to_string()
        })
        .await?;

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
            .await?;

        // Delete from search engine
        let index_project = meili.index(showtimes_search::models::Project::index_name());

        let task_del = index_project.delete_documents(&all_project_ids).await?;
        task_del.wait_for_completion(meili, None, None).await?;
    }

    // Delete assets
    storages
        .directory_delete(server.id, None, Some(showtimes_fs::FsFileKind::Images))
        .await?;

    // Delete from DB
    let srv_handler = ServerHandler::new(db);
    srv_handler.delete(&server).await?;

    // Delete from search engine
    let srv_index = meili.index(showtimes_search::models::Server::index_name());
    srv_index.delete_document(server.id.to_string()).await?;

    Ok(OkResponse::ok("Server deleted"))
}
