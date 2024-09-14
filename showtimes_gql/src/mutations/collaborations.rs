use async_graphql::{dataloader::DataLoader, Error, ErrorExtensions, InputObject};
use showtimes_db::{CollaborationInviteHandler, DatabaseShared};
use showtimes_search::SearchClientShared;

use crate::{
    data_loader::{ProjectDataLoader, ServerDataLoader},
    models::{collaborations::CollaborationInviteGQL, prelude::UlidGQL},
};

/// The user input object on what to update
///
/// All fields are optional
#[derive(InputObject)]
pub struct CollaborationRequestInputGQL {
    /// The original project request
    project: UlidGQL,
    /// The target project request
    ///
    /// If not provided, this will duplicate the original
    /// project information into this server
    #[graphql(name = "targetProject")]
    target_project: Option<UlidGQL>,
    /// The target server request
    #[graphql(name = "targetServer")]
    target_server: UlidGQL,
}

async fn check_permissions(
    ctx: &async_graphql::Context<'_>,
    user: &showtimes_db::m::User,
    id: showtimes_shared::ulid::Ulid,
) -> async_graphql::Result<showtimes_db::m::Server> {
    let srv_loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();

    let srv = srv_loader.load_one(id).await?;
    if srv.is_none() {
        return Err(Error::new("Server not found").extend_with(|_, e| {
            e.set("id", id.to_string());
            e.set("reason", "invalid_server");
        }));
    }

    let srv = srv.unwrap();
    let find_user = srv.owners.iter().find(|o| o.id == user.id);

    match (find_user, user.kind) {
        (Some(user), showtimes_db::m::UserKind::User) => {
            // Check if we are allowed to do collaboration
            if user.privilege < showtimes_db::m::UserPrivilege::Manager {
                Err(
                    Error::new("User not allowed to manage collaboration").extend_with(|_, e| {
                        e.set("id", id.to_string());
                        e.set("reason", "invalid_privilege");
                    }),
                )
            } else {
                Ok(srv)
            }
        }
        (None, showtimes_db::m::UserKind::User) => Err(Error::new(
            "User not allowed to manage collaboration",
        )
        .extend_with(|_, e| {
            e.set("id", id.to_string());
            e.set("reason", "invalid_user");
        })),
        _ => {
            // Allow anyone to manage collaboration
            Ok(srv)
        }
    }
}

pub async fn mutate_colaborations_initiate(
    ctx: &async_graphql::Context<'_>,
    user: showtimes_db::m::User,
    input: CollaborationRequestInputGQL,
) -> async_graphql::Result<CollaborationInviteGQL> {
    let prj_loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();
    let db = ctx.data_unchecked::<DatabaseShared>();
    let meili = ctx.data_unchecked::<SearchClientShared>();

    let project = prj_loader.load_one(*input.project).await?;
    if project.is_none() {
        return Err(Error::new("Project not found").extend_with(|_, e| {
            e.set("id", input.project.to_string());
            e.set("reason", "invalid_project");
        }));
    }

    let project = project.unwrap();

    check_permissions(ctx, &user, project.creator).await?;

    let target_proj_id = match input.target_project {
        Some(target) => match prj_loader.load_one(*target).await? {
            Some(prj) => {
                if prj.creator != *input.target_server {
                    return Err(Error::new("Target project has invalid owner").extend_with(
                        |_, e| {
                            e.set("id", target.to_string());
                            e.set("expect_server", input.target_server.to_string());
                            e.set("reason", "invalid_project");
                        },
                    ));
                } else {
                    Some(prj.id)
                }
            }
            None => {
                return Err(Error::new("Target project not found").extend_with(|_, e| {
                    e.set("id", target.to_string());
                    e.set("reason", "invalid_project");
                }));
            }
        },
        None => None,
    };

    let source_invite =
        showtimes_db::m::ServerCollaborationInviteSource::new(project.creator, project.id);
    let target_invite = match target_proj_id {
        Some(target) => showtimes_db::m::ServerCollaborationInviteTarget::new_with_project(
            *input.target_server,
            target,
        ),
        None => showtimes_db::m::ServerCollaborationInviteTarget::new(*input.target_server),
    };

    // Save the invite
    let mut collab_invite =
        showtimes_db::m::ServerCollaborationInvite::new(source_invite, target_invite);

    let invite_handler = CollaborationInviteHandler::new(db);
    invite_handler.save(&mut collab_invite, None).await?;

    // Save in search index
    let invite_search = showtimes_search::models::ServerCollabInvite::from(collab_invite.clone());
    invite_search.update_document(meili).await?;

    Ok(collab_invite.into())
}
