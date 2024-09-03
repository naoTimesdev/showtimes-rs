use std::sync::Arc;

use async_graphql::{dataloader::DataLoader, Error, ErrorExtensions, InputObject, Upload};
use showtimes_db::{m::UserKind, DatabaseShared, ServerHandler};
use showtimes_fs::FsPool;
use showtimes_search::SearchClientShared;
use tokio::io::AsyncSeekExt;

use crate::{
    data_loader::ServerDataLoader,
    models::{prelude::UlidGQL, servers::ServerGQL, users::UserKindGQL},
};

use super::{IntegrationActionGQL, IntegrationInputGQL, IntegrationValidator};

/// The server input object on what to update
///
/// All fields are optional
#[derive(InputObject)]
pub struct ServerInputGQL {
    /// The server name
    #[graphql(validator(min_length = 5, max_length = 128))]
    name: Option<String>,
    /// The list of integration to add, update, or remove
    #[graphql(validator(custom = "IntegrationValidator::new()"))]
    integrations: Option<Vec<IntegrationInputGQL>>,
    /// The server avatar
    avatar: Option<Upload>,
}

impl ServerInputGQL {
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

pub async fn mutate_servers_update(
    ctx: &async_graphql::Context<'_>,
    id: UlidGQL,
    user: showtimes_db::m::User,
    input: ServerInputGQL,
) -> async_graphql::Result<ServerGQL> {
    if !input.is_any_set() {
        return Err(Error::new("No fields to update"));
    }

    let loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();
    let db = ctx.data_unchecked::<DatabaseShared>();
    let storages = ctx.data_unchecked::<Arc<FsPool>>();
    let meili = ctx.data_unchecked::<SearchClientShared>();

    let server = loader.load_one(*id).await?.ok_or_else(|| {
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

    // Anything below Admin is disallowed
    if user_owner.privilege < showtimes_db::m::UserPrivilege::Owner {
        return Err(
            Error::new("User does not have permission to update the server").extend_with(|_, e| {
                e.set("id", id.to_string());
                e.set("user", user.id.to_string());
                e.set("privilege", user_owner.privilege.to_string());
            }),
        );
    }

    // Do update
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
