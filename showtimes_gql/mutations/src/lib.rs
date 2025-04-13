#![warn(missing_docs, clippy::empty_docs, rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../../README.md")]

use std::sync::Arc;

use async_graphql::{Context, Object, dataloader::DataLoader};

mod collaborations;
mod common;
mod projects;
mod rss;
mod servers;
mod users;

pub(crate) use common::*;

use showtimes_db::m::APIKeyCapability;
use showtimes_gql_common::{
    DateTimeTZGQL, GQLErrorCode, GQLErrorExt, OkResponse, Orchestrator, UserKindGQL,
    data_loader::UserDataLoader,
    errors::GQLError,
    guard::{APIKeyVerify, AuthUserAndAPIKeyGuard, AuthUserMinimumGuard},
};
use showtimes_gql_events_models::rss::RSSFeedRenderedGQL;
use showtimes_gql_models::{
    collaborations::{CollaborationInviteGQL, CollaborationSyncGQL},
    projects::ProjectGQL,
    rss::RSSFeedGQL,
    servers::{ServerGQL, ServerPremiumGQL},
    users::{UserGQL, UserSessionGQL},
};
use showtimes_session::{ShowtimesUserSession, manager::SharedSessionManager};

/// The main Mutation Root type for the GraphQL schema. This is where all the mutation are defined.
pub struct MutationRoot;

/// The main Mutation Root type for the GraphQL schema. This is where all the mutation are defined.
#[Object]
impl MutationRoot {
    /// Authorize Discord OAuth2 token and state that was returned from the OAuth2 redirect
    async fn auth(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The OAuth2 token/code returned from Discord")] token: String,
        #[graphql(desc = "The OAuth2 state")] state: String,
    ) -> async_graphql::Result<UserSessionGQL> {
        crate::users::mutate_users_authenticate(ctx, token, state).await
    }

    /// Disconnect/logout from Showtimes, this can also be used to revoke OAuth2 token
    #[graphql(guard = "AuthUserMinimumGuard::new(UserKindGQL::User)")]
    async fn disconnect(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Revoke specific token, this only works for Owner auth")] token: Option<
            String,
        >,
    ) -> async_graphql::Result<OkResponse> {
        let jwt = ctx.data_unchecked::<ShowtimesUserSession>();
        let sessions = ctx.data_unchecked::<SharedSessionManager>();

        match (token, jwt.get_claims().get_audience()) {
            (_, showtimes_session::ShowtimesAudience::User) => {
                sessions
                    .lock()
                    .await
                    .remove_session(jwt.get_token())
                    .await
                    .extend_error(GQLErrorCode::SessionDeleteError, |f_ctx| {
                        f_ctx.set("token", jwt.get_token());
                    })?;

                Ok(OkResponse::ok("Successfully logged out"))
            }
            (Some(token), showtimes_session::ShowtimesAudience::MasterKey) => {
                sessions
                    .lock()
                    .await
                    .remove_session(&token)
                    .await
                    .extend_error(GQLErrorCode::SessionDeleteError, |f_ctx| {
                        f_ctx.set("token", &token);
                        f_ctx.set("is_master", true);
                    })?;

                Ok(OkResponse::ok("Successfully revoked token"))
            }
            _ => {
                // Just stub for now
                Ok(OkResponse::ok("Successfully disconnected"))
            }
        }
    }

    /// Create a new server in Showtimes
    #[graphql(
        name = "createServer",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::ManageServers))"
    )]
    async fn create_server(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The server information to be created")]
        input: servers::ServerCreateInputGQL,
    ) -> async_graphql::Result<ServerGQL> {
        servers::mutate_servers_create(ctx, input).await
    }

    /// Create a new project in Showtimes
    #[graphql(
        name = "createProject",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::ManageProjects))"
    )]
    async fn create_project(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The server ID for the project")] id: showtimes_gql_common::UlidGQL,
        #[graphql(desc = "The project information to be created")]
        input: projects::ProjectCreateInputGQL,
    ) -> async_graphql::Result<ProjectGQL> {
        let user = ctx.data_unchecked::<showtimes_db::m::User>();

        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        projects::mutate_projects_create(ctx, user_behalf.unwrap_or(user.clone()), id, input).await
    }

    /// Create a new RSS feed on Showtimes
    #[graphql(
        name = "createRssFeed",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::ManageRSS))"
    )]
    async fn create_rss_feed(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The input to create the RSS feed")] input: rss::RSSFeedCreateInputGQL,
    ) -> async_graphql::Result<RSSFeedGQL> {
        rss::mutate_rss_feed_create(ctx, input).await
    }

    /// Update user information
    #[graphql(
        name = "updateUser",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::ManageUsers))"
    )]
    async fn update_user(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The user ID to update, when NOT provided will use the current user")] id: Option<showtimes_gql_common::UlidGQL>,
        #[graphql(desc = "The user information to update")] input: users::UserInputGQL,
    ) -> async_graphql::Result<UserGQL> {
        let user = ctx.data_unchecked::<showtimes_db::m::User>();

        let requested = users::UserRequester::new(user.clone());
        let requested = if let Some(id) = id {
            requested.with_id(*id)
        } else {
            requested
        };

        users::mutate_users_update(ctx, requested, input).await
    }

    /// Update server information
    #[graphql(
        name = "updateServer",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::ManageServers))"
    )]
    async fn update_server(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The server ID to update")] id: showtimes_gql_common::UlidGQL,
        #[graphql(desc = "The server information to update")] input: servers::ServerUpdateInputGQL,
    ) -> async_graphql::Result<ServerGQL> {
        servers::mutate_servers_update(ctx, id, input).await
    }

    /// Update project information
    #[graphql(
        name = "updateProject",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::ManageProjects))"
    )]
    async fn update_project(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The project ID to update")] id: showtimes_gql_common::UlidGQL,
        #[graphql(desc = "The project information to update")]
        input: projects::ProjectUpdateInputGQL,
    ) -> async_graphql::Result<ProjectGQL> {
        let user = ctx.data_unchecked::<showtimes_db::m::User>();

        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        projects::mutate_projects_update(ctx, user_behalf.unwrap_or(user.clone()), id, input).await
    }

    /// Add new episode automatically to a project
    ///
    /// This will use the last episode as the base for the new episode
    #[graphql(
        name = "updateProjectProgressAddAuto",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::ManageProjects))"
    )]
    async fn update_project_progress_auto_add(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The project ID to update")] id: showtimes_gql_common::UlidGQL,
        #[graphql(
            desc = "The total number of episodes to add, minimum of 1 and maximum of 100",
            validator(minimum = 1, maximum = 100)
        )]
        total: u32,
    ) -> async_graphql::Result<ProjectGQL> {
        let user = ctx.data_unchecked::<showtimes_db::m::User>();

        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        projects::mutate_projects_episode_add_auto(
            ctx,
            user_behalf.unwrap_or(user.clone()),
            id,
            total.into(),
        )
        .await
    }

    /// Add new episode manually to a project
    ///
    /// You will need to provide each episode information
    #[graphql(
        name = "updateProjectProgressAdd",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::ManageProjects))"
    )]
    async fn update_project_progress_add(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The project ID to update")] id: showtimes_gql_common::UlidGQL,
        #[graphql(
            desc = "The new episodes to be added, minimum of 1 and maximum of 100",
            validator(min_items = 1, max_items = 100)
        )]
        episodes: Vec<projects::ProgressCreateInputGQL>,
    ) -> async_graphql::Result<ProjectGQL> {
        let user = ctx.data_unchecked::<showtimes_db::m::User>();

        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        projects::mutate_projects_episode_add_manual(
            ctx,
            user_behalf.unwrap_or(user.clone()),
            id,
            &episodes,
        )
        .await
    }

    /// Add new episode automatically to a project
    ///
    /// This will use the last episode as the base for the new episode
    #[graphql(
        name = "updateProjectProgressRemove",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::ManageProjects))"
    )]
    async fn update_project_progress_remove(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The project ID to update")] id: showtimes_gql_common::UlidGQL,
        #[graphql(
            desc = "The episodes to delete, minimum of 1 and maximum of 100",
            validator(min_items = 1, max_items = 100)
        )]
        episodes: Vec<u64>,
    ) -> async_graphql::Result<ProjectGQL> {
        let user = ctx.data_unchecked::<showtimes_db::m::User>();

        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        projects::mutate_projects_episode_remove(
            ctx,
            user_behalf.unwrap_or(user.clone()),
            id,
            &episodes,
        )
        .await
    }

    /// Update a RSS feed on Showtimes
    #[graphql(
        name = "updateRssFeed",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::ManageRSS))"
    )]
    async fn update_rss_feed(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The RSS feed ID to update")] id: showtimes_gql_common::UlidGQL,
        #[graphql(desc = "The input to update the RSS feed")] input: rss::RSSFeedUpdateInputGQL,
    ) -> async_graphql::Result<RSSFeedGQL> {
        rss::mutate_rss_feed_update(ctx, id, input).await
    }

    /// Initiate a collaboration between projects
    #[graphql(
        name = "collaborateProjects",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::ManageCollaboration))"
    )]
    async fn collaborate_projects(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The collaboration information to be created")]
        input: collaborations::CollaborationRequestInputGQL,
    ) -> async_graphql::Result<CollaborationInviteGQL> {
        let user = ctx.data_unchecked::<showtimes_db::m::User>();

        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        collaborations::mutate_collaborations_initiate(
            ctx,
            user_behalf.unwrap_or(user.clone()),
            input,
        )
        .await
    }

    /// Accept a collaboration request between projects
    #[graphql(
        name = "collaborateAccept",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::ManageCollaboration))"
    )]
    async fn collaborate_accept(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The collaboration ID to accept")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<CollaborationSyncGQL> {
        let user = ctx.data_unchecked::<showtimes_db::m::User>();

        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        collaborations::mutate_collaborations_accept(ctx, user_behalf.unwrap_or(user.clone()), id)
            .await
    }

    /// Deny a collaboration request between projects
    #[graphql(
        name = "collaborateDeny",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::ManageCollaboration))"
    )]
    async fn collaborate_deny(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The collaboration ID to deny")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<OkResponse> {
        let user = ctx.data_unchecked::<showtimes_db::m::User>();

        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        collaborations::mutate_collaborations_cancel(
            ctx,
            user_behalf.unwrap_or(user.clone()),
            id,
            true,
        )
        .await
    }

    /// Retract a collaboration request between projects
    #[graphql(
        name = "collaborateRetract",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::ManageCollaboration))"
    )]
    async fn collaborate_retract(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The collaboration ID to retract")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<OkResponse> {
        let user = ctx.data_unchecked::<showtimes_db::m::User>();

        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        collaborations::mutate_collaborations_cancel(
            ctx,
            user_behalf.unwrap_or(user.clone()),
            id,
            false,
        )
        .await
    }

    /// Delete or unlink a collaboration between projects
    #[graphql(
        name = "collaborateDelete",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::ManageCollaboration))"
    )]
    async fn collaborate_delete(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The collaboration ID target")] id: showtimes_gql_common::UlidGQL,
        #[graphql(desc = "The target project to delete or remove")]
        target: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<OkResponse> {
        let user = ctx.data_unchecked::<showtimes_db::m::User>();

        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        collaborations::mutate_collaborations_unlink(
            ctx,
            user_behalf.unwrap_or(user.clone()),
            id,
            target,
        )
        .await
    }

    /// Delete a project from Showtimes
    #[graphql(
        name = "deleteProject",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::DeleteProjects))"
    )]
    async fn delete_project(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The project ID to delete")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<OkResponse> {
        let user = ctx.data_unchecked::<showtimes_db::m::User>();

        let user_behalf = match ctx.data_unchecked::<Orchestrator>() {
            Orchestrator::Standalone => None,
            other => {
                // Only allow if the user is type is Admin or greater
                if user.kind >= showtimes_db::m::UserKind::Admin {
                    other.to_user(ctx).await?
                } else {
                    None
                }
            }
        };

        projects::mutate_projects_delete(ctx, user_behalf.unwrap_or(user.clone()), id).await
    }

    /// Delete a server from Showtimes
    #[graphql(
        name = "deleteServer",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::DeleteServers))"
    )]
    async fn delete_server(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The server ID to delete")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<OkResponse> {
        servers::mutate_servers_delete(ctx, id).await
    }

    /// Delete a RSS feed on Showtimes
    #[graphql(
        name = "deleteRssFeed",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::Specific(APIKeyCapability::ManageRSS))"
    )]
    async fn delete_rss_feed(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The RSS feed ID to delete")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<OkResponse> {
        rss::mutate_rss_feed_delete(ctx, id).await
    }

    /// Create a session for another user.
    ///
    /// This is mainly used by other services to orchestrate Showtimes on behalf of the user.
    ///
    /// Only available for Admin users.
    #[graphql(
        name = "createSession",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::NotAllowed)"
    )]
    async fn create_session(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The user ID to create session for")] id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<UserSessionGQL> {
        let loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();
        let session = ctx.data_unchecked::<SharedSessionManager>();
        let config = ctx.data_unchecked::<Arc<showtimes_shared::config::Config>>();
        let jwt = ctx.data_unchecked::<showtimes_session::SharedSigner>();
        let user = loader.load_one(*id).await?.ok_or_else(|| {
            GQLError::new("User not found", GQLErrorCode::UserNotFound)
                .extend(|e| e.set("id", id.to_string()))
        })?;

        // Create actual session
        let (claims, _) =
            showtimes_session::create_session(user.id, config.jwt.get_expiration() as i64, jwt)?;

        // We don't create refresh token session for this custom orchestration.
        let mut sess_mutex = session.lock().await;
        sess_mutex
            .set_session(claims.get_token(), claims.get_claims())
            .await
            .extend_error(GQLErrorCode::SessionCreateError, |f_ctx| {
                f_ctx.set("id", id.to_string());
            })?;
        drop(sess_mutex);

        Ok(UserSessionGQL::new(&user, claims.get_token()))
    }

    /// Create or update a server premium status.
    ///
    /// There is some gotcha's with the implementation:
    /// - If there is no current active premium status, this will create a new one.
    /// - If there is an existing one, it will be either:
    ///   - Updated to the `ends_at` if it is newer than the existing one.
    ///   - Extended by the `ends_at` difference with current time if it is older than the existing one.
    #[graphql(
        name = "createOrUpdateServerPremium",
        guard = "AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn create_or_update_server_premium(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The server ID to create/update premium status for")]
        id: showtimes_gql_common::UlidGQL,
        #[graphql(desc = "The end date of the premium status")] ends_at: DateTimeTZGQL,
    ) -> async_graphql::Result<ServerPremiumGQL> {
        servers::mutate_servers_premium_create(ctx, id, ends_at).await
    }

    /// Delete a server premium status.
    #[graphql(
        name = "deleteServerPremium",
        guard = "AuthUserMinimumGuard::new(UserKindGQL::Admin)"
    )]
    async fn delete_server_premium(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The server ID to delete premium status for")]
        id: showtimes_gql_common::UlidGQL,
    ) -> async_graphql::Result<OkResponse> {
        servers::mutate_servers_premium_delete(ctx, id).await
    }

    /// Preview RSS feed template or display.
    ///
    /// This will use the latest data from the RSS feed to generate the preview.
    /// If there is no latest data
    #[graphql(
        name = "previewRssFeed",
        guard = "AuthUserAndAPIKeyGuard::new(UserKindGQL::User, APIKeyVerify::AllowAny)"
    )]
    async fn preview_rss_feed(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The RSS feed ID to preview")] id: showtimes_gql_common::UlidGQL,
        #[graphql(desc = "The input to preview")] input: rss::RSSFeedDisplayPreviewInputGQL,
    ) -> async_graphql::Result<Option<RSSFeedRenderedGQL>> {
        rss::mutate_rss_feed_preview(ctx, id, input).await
    }
}
