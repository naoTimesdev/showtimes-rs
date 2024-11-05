use async_graphql::{dataloader::DataLoader, Error, ErrorExtensions, InputObject};
use showtimes_db::{
    m::{ServerCollaborationSyncTarget, UserKind},
    mongodb::bson::doc,
    CollaborationInviteHandler, DatabaseShared,
};
use showtimes_search::SearchClientShared;

use showtimes_gql_common::{
    data_loader::{ProjectDataLoader, ServerDataLoader, ServerSyncLoader},
    OkResponse, UlidGQL,
};
use showtimes_gql_models::collaborations::{CollaborationInviteGQL, CollaborationSyncGQL};

use crate::execute_search_events;

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

pub async fn mutate_collaborations_initiate(
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
    let invite_clone = collab_invite.clone();
    let meili_clone = meili.clone();
    let task_search = tokio::task::spawn(async move {
        let invite_search = showtimes_search::models::ServerCollabInvite::from(invite_clone);
        invite_search.update_document(&meili_clone).await
    });
    // Save in event
    let task_events = ctx
        .data_unchecked::<showtimes_events::SharedSHClickHouse>()
        .create_event_async(
            showtimes_events::m::EventKind::CollaborationCreated,
            showtimes_events::m::CollabCreatedEvent::from(&collab_invite),
            if user.kind == UserKind::Owner {
                None
            } else {
                Some(user.id.to_string())
            },
        );

    execute_search_events(task_search, task_events).await?;

    Ok(collab_invite.into())
}

pub async fn mutate_collaborations_accept(
    ctx: &async_graphql::Context<'_>,
    user: showtimes_db::m::User,
    invite: UlidGQL,
) -> async_graphql::Result<CollaborationSyncGQL> {
    let db = ctx.data_unchecked::<DatabaseShared>();
    let meili = ctx.data_unchecked::<SearchClientShared>();
    let prj_loader = ctx.data_unchecked::<DataLoader<ProjectDataLoader>>();

    let invite_db = showtimes_db::CollaborationInviteHandler::new(db);
    let invite_data = invite_db
        .find_by_id(&invite.to_string())
        .await?
        .ok_or_else(|| {
            Error::new("Collaboration invite not found").extend_with(|_, e| {
                e.set("id", invite.to_string());
                e.set("reason", "invalid_invite");
            })
        })?;

    let target_srv = check_permissions(ctx, &user, invite_data.target.server).await?;

    let orig_proj = prj_loader
        .load_one(invite_data.source.project)
        .await?
        .ok_or_else(|| {
            Error::new("Original/source project not found").extend_with(|_, e| {
                e.set("id", invite_data.source.project.to_string());
                e.set("reason", "invalid_project");
            })
        })?;

    // Check done, see if we can just use the existing project or should we duplicate
    let mut target_proj = if let Some(project_id) = invite_data.target.project {
        let project = prj_loader.load_one(project_id).await?;

        match project {
            None => orig_proj.duplicate(target_srv.id),
            Some(target_proj) => target_proj,
        }
    } else {
        orig_proj.duplicate(target_srv.id)
    };

    // Save the project to DB first for target
    let prj_handler = showtimes_db::ProjectHandler::new(db);
    prj_handler.save(&mut target_proj, None).await?;

    // Save to search index
    let prj_search = showtimes_search::models::Project::from(target_proj.clone());
    prj_search.update_document(meili).await?;

    // Find any pre-existing sync
    let sync_handler = showtimes_db::CollaborationSyncHandler::new(db);
    let mut sync_ss = sync_handler
        .find_by(doc! {
            "projects.project": orig_proj.id.to_string(),
        })
        .await?;

    let sync_mut = sync_ss.as_mut();

    // Match, update the list then save
    let sync_gql: CollaborationSyncGQL = if let Some(sync) = sync_mut {
        sync.projects
            .push(ServerCollaborationSyncTarget::from(target_proj));

        // Update DB
        sync_handler.save(sync, None).await?;

        // Save in search index
        let sync_clone = sync.clone();
        let meili_clone = meili.clone();
        let task_search = tokio::task::spawn(async move {
            let sync_search = showtimes_search::models::ServerCollabSync::from(sync_clone);
            sync_search.update_document(&meili_clone).await
        });
        // Save in event
        let task_events = ctx
            .data_unchecked::<showtimes_events::SharedSHClickHouse>()
            .create_event_async(
                showtimes_events::m::EventKind::CollaborationAccepted,
                showtimes_events::m::CollabAcceptedEvent::new(invite_data.id, sync.id),
                if user.kind == UserKind::Owner {
                    None
                } else {
                    Some(user.id.to_string())
                },
            );

        execute_search_events(task_search, task_events).await?;

        sync.clone().into()
    } else {
        // Create a new sync
        let src_sync = showtimes_db::m::ServerCollaborationSyncTarget::from(orig_proj);
        let target_sync = showtimes_db::m::ServerCollaborationSyncTarget::from(target_proj);

        let mut sync = showtimes_db::m::ServerCollaborationSync::new(vec![src_sync, target_sync]);

        // Save to DB
        sync_handler.save(&mut sync, None).await?;

        // Save in search index
        let sync_clone = sync.clone();
        let meili_clone = meili.clone();
        let task_search = tokio::task::spawn(async move {
            let sync_search = showtimes_search::models::ServerCollabSync::from(sync_clone);
            sync_search.update_document(&meili_clone).await
        });
        // Save in event
        let task_events = ctx
            .data_unchecked::<showtimes_events::SharedSHClickHouse>()
            .create_event_async(
                showtimes_events::m::EventKind::CollaborationAccepted,
                showtimes_events::m::CollabAcceptedEvent::new(invite_data.id, sync.id),
                if user.kind == UserKind::Owner {
                    None
                } else {
                    Some(user.id.to_string())
                },
            );

        execute_search_events(task_search, task_events).await?;

        sync.clone().into()
    };

    // Delete invite
    invite_db.delete(&invite_data).await?;
    // Remove from search index
    let invite_search = showtimes_search::models::ServerCollabInvite::from(invite_data.clone());
    invite_search.delete_document(meili).await?;

    Ok(sync_gql)
}

pub async fn mutate_collaborations_cancel(
    ctx: &async_graphql::Context<'_>,
    user: showtimes_db::m::User,
    invite: UlidGQL,
    is_deny: bool,
) -> async_graphql::Result<OkResponse> {
    let db = ctx.data_unchecked::<DatabaseShared>();
    let meili = ctx.data_unchecked::<SearchClientShared>();

    let invite_db = showtimes_db::CollaborationInviteHandler::new(db);
    let invite_data = invite_db
        .find_by_id(&invite.to_string())
        .await?
        .ok_or_else(|| {
            Error::new("Collaboration invite not found").extend_with(|_, e| {
                e.set("id", invite.to_string());
                e.set("reason", "invalid_invite");
            })
        })?;

    // Check target server permissions
    let server_id = if is_deny {
        invite_data.target.server
    } else {
        invite_data.source.server
    };
    check_permissions(ctx, &user, server_id).await?;

    // Deny the invite
    invite_db.delete(&invite_data).await?;

    // Remove from search index
    let meili_clone = meili.clone();
    let invite_clone = invite_data.clone();

    let task_search = tokio::task::spawn(async move {
        let invite_search = showtimes_search::models::ServerCollabInvite::from(invite_clone);
        invite_search.delete_document(&meili_clone).await
    });

    // Save in event
    let event_ch = ctx.data_unchecked::<showtimes_events::SharedSHClickHouse>();
    let task_events = if is_deny {
        event_ch.create_event_async(
            showtimes_events::m::EventKind::CollaborationRejected,
            showtimes_events::m::CollabRejectedEvent::from(&invite_data),
            if user.kind == UserKind::Owner {
                None
            } else {
                Some(user.id.to_string())
            },
        )
    } else {
        event_ch.create_event_async(
            showtimes_events::m::EventKind::CollaborationRetracted,
            showtimes_events::m::CollabRetractedEvent::from(&invite_data),
            if user.kind == UserKind::Owner {
                None
            } else {
                Some(user.id.to_string())
            },
        )
    };

    execute_search_events(task_search, task_events).await?;

    if is_deny {
        Ok(OkResponse::ok("Invite denied"))
    } else {
        Ok(OkResponse::ok("Invite retracted"))
    }
}

pub async fn mutate_collaborations_unlink(
    ctx: &async_graphql::Context<'_>,
    user: showtimes_db::m::User,
    sync_id: UlidGQL,
    initiator: UlidGQL,
) -> async_graphql::Result<OkResponse> {
    let loader = ctx.data_unchecked::<DataLoader<ServerSyncLoader>>();
    let db = ctx.data_unchecked::<DatabaseShared>();

    let sync_handler = showtimes_db::CollaborationSyncHandler::new(db);
    let sync = loader.load_one(*sync_id).await?;

    if sync.is_none() {
        return Err(
            Error::new("Collaboration sync not found").extend_with(|_, e| {
                e.set("id", sync_id.to_string());
                e.set("reason", "invalid_sync");
            }),
        );
    }

    let mut sync = sync.unwrap();

    // Initiator is the project ID, find the project
    let project_sync = sync.get_and_remove(*initiator);

    if project_sync.is_none() {
        return Err(
            Error::new("Project not found in collaboration data").extend_with(|_, e| {
                e.set("id", initiator.to_string());
                e.set("reason", "invalid_project");
            }),
        );
    }

    let project_sync = project_sync.unwrap();

    // Check permissions
    check_permissions(ctx, &user, project_sync.server).await?;

    // Check if we need to delete the sync
    if sync.length() < 2 {
        // Delete the sync
        sync_handler.delete(&sync).await?;

        // Remove from search index
        let sync_clone = sync.clone();
        let meili_clone = ctx.data_unchecked::<SearchClientShared>().clone();
        let task_search = tokio::task::spawn(async move {
            let sync_search = showtimes_search::models::ServerCollabSync::from(sync_clone);
            sync_search.delete_document(&meili_clone).await
        });

        // Save in event
        let task_events = ctx
            .data_unchecked::<showtimes_events::SharedSHClickHouse>()
            .create_event_async(
                showtimes_events::m::EventKind::CollaborationDeleted,
                showtimes_events::m::CollabDeletedEvent::new(sync.id, &project_sync, true),
                if user.kind == UserKind::Owner {
                    None
                } else {
                    Some(user.id.to_string())
                },
            );

        execute_search_events(task_search, task_events).await?;
    } else {
        sync_handler.save(&mut sync, None).await?;

        // Save in search index
        let sync_clone = sync.clone();
        let meili_clone = ctx.data_unchecked::<SearchClientShared>().clone();
        let task_search = tokio::task::spawn(async move {
            let sync_search = showtimes_search::models::ServerCollabSync::from(sync_clone);
            sync_search.update_document(&meili_clone).await
        });

        // Save in event
        let task_events = ctx
            .data_unchecked::<showtimes_events::SharedSHClickHouse>()
            .create_event_async(
                showtimes_events::m::EventKind::CollaborationDeleted,
                showtimes_events::m::CollabDeletedEvent::new(sync.id, &project_sync, false),
                if user.kind == UserKind::Owner {
                    None
                } else {
                    Some(user.id.to_string())
                },
            );

        execute_search_events(task_search, task_events).await?;
    }

    Ok(OkResponse::ok("Collaboration deleted or unlinked"))
}
