use std::sync::{Arc, OnceLock};

use async_graphql_axum::{GraphQLProtocol, GraphQLRequest, GraphQLResponse, GraphQLWebSocket};
use axum::{
    extract::{State, WebSocketUpgrade},
    http::HeaderMap,
    response::{Html, IntoResponse, Response},
};
use showtimes_db::DatabaseShared;
use showtimes_gql_common::{
    data_loader, graphiql_plugin_explorer, Data as GQLData, Error as GQLError, GQLDataLoaderWhere,
    GQLErrorCode, GQLResponse, GQLServerError, GraphiQLSource, ALL_WEBSOCKET_PROTOCOLS,
};
use showtimes_gql_mutations::MutationRoot;
use showtimes_gql_queries::QueryRoot;
use showtimes_gql_subscriptions::SubscriptionRoot;
use showtimes_session::{manager::SessionKind, oauth2::discord::DiscordClient};
use showtimes_shared::Config;

use crate::state::SharedShowtimesState;

/// The main schema for our GraphQL server.
///
/// Wraps [`QueryRoot`], [`MutationRoot`], and [`SubscriptionRoot`] types.
pub type ShowtimesGQLSchema =
    showtimes_gql_common::Schema<QueryRoot, MutationRoot, SubscriptionRoot>;
pub const GRAPHQL_ROUTE: &str = "/graphql";
pub const GRAPHQL_WS_ROUTE: &str = "/graphql/ws";

static DISCORD_CLIENT: OnceLock<Arc<DiscordClient>> = OnceLock::new();
static STUBBED_OWNER: OnceLock<showtimes_db::m::User> = OnceLock::new();
static GRAPHQL_SDL: OnceLock<String> = OnceLock::new();

/// Create the GraphQL schema
pub fn create_schema(db_pool: &DatabaseShared) -> ShowtimesGQLSchema {
    showtimes_gql_common::Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .extension(showtimes_gql_common::Tracing)
        .data(showtimes_gql_common::DataLoader::new(
            data_loader::UserDataLoader::new(db_pool),
            tokio::spawn,
        ))
        .data(showtimes_gql_common::DataLoader::new(
            data_loader::ProjectDataLoader::new(db_pool),
            tokio::spawn,
        ))
        .data(showtimes_gql_common::DataLoader::new(
            data_loader::ServerDataLoader::new(db_pool),
            tokio::spawn,
        ))
        .data(showtimes_gql_common::DataLoader::new(
            data_loader::ServerSyncLoader::new(db_pool),
            tokio::spawn,
        ))
        .data(showtimes_gql_common::DataLoader::new(
            data_loader::ServerInviteLoader::new(db_pool),
            tokio::spawn,
        ))
        .data(showtimes_gql_common::DataLoader::new(
            data_loader::RSSFeedLoader::new(db_pool),
            tokio::spawn,
        ))
        .data(showtimes_gql_common::DataLoader::new(
            data_loader::ServerPremiumLoader::new(db_pool),
            tokio::spawn,
        ))
        .finish()
}

pub async fn graphql_sdl(State(state): State<SharedShowtimesState>) -> impl IntoResponse {
    // Cache the SDL since it only change between compilation
    let sdl_data = GRAPHQL_SDL.get_or_init(|| state.schema.sdl());

    // Return the SDL as plain text
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "text/plain".parse().unwrap());
    (headers, sdl_data.clone())
}

pub async fn graphql_playground() -> impl IntoResponse {
    let plugins = vec![graphiql_plugin_explorer()];
    let source = GraphiQLSource::build()
        .endpoint(GRAPHQL_ROUTE)
        .subscription_endpoint(GRAPHQL_WS_ROUTE)
        .plugins(&plugins)
        .title("GraphiQL Playground")
        .finish();

    Html(source)
}

fn get_token_or_bearer(headers: &HeaderMap, config: &Config) -> Option<(SessionKind, String)> {
    headers.get("Authorization").and_then(|value| {
        value.to_str().ok().and_then(|value| {
            if value.starts_with("Bearer ") {
                value
                    .strip_prefix("Bearer ")
                    .map(|token| (SessionKind::Bearer, token.to_string()))
            } else if value.starts_with("Token ") {
                value.strip_prefix("Token ").map(|token| {
                    if token == config.master_key {
                        (SessionKind::MasterKey, token.to_string())
                    } else {
                        (SessionKind::APIKey, token.to_string())
                    }
                })
            } else {
                None
            }
        })
    })
}

fn get_orchestrator(headers: &HeaderMap) -> showtimes_gql_common::Orchestrator {
    match headers.get("x-orchestrator") {
        None => showtimes_gql_common::Orchestrator::Standalone,
        Some(value) => match value.to_str() {
            Ok(header) => showtimes_gql_common::Orchestrator::from_header(Some(header)),
            Err(_) => showtimes_gql_common::Orchestrator::Standalone,
        },
    }
}

/// The main GraphQL handler
///
/// This handler will handle all GraphQL requests, it will also handle the authentication
pub async fn graphql_handler(
    State(state): State<SharedShowtimesState>,
    headers: HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let mut req = req.into_inner();
    req = req.data(state.db.clone());
    req = req.data(state.config.clone());

    let discord_client = DISCORD_CLIENT.get_or_init(|| {
        Arc::new(DiscordClient::new(
            state.config.discord.client_id.clone(),
            state.config.discord.client_secret.clone(),
        ))
    });

    req = req.data(discord_client.clone());
    req = req.data(state.meili.clone());
    req = req.data(state.clickhouse.clone());
    req = req.data(state.session.clone());
    req = req.data(state.storage.clone());
    req = req.data(state.anilist_provider.clone());
    if let Some(tmdb_provider) = state.tmdb_provider.as_ref() {
        req = req.data(tmdb_provider.clone());
    }
    if let Some(vndb_provider) = state.vndb_provider.as_ref() {
        req = req.data(vndb_provider.clone());
    }

    if let Some((kind, token)) = get_token_or_bearer(&headers, &state.config) {
        match state.session.lock().await.get_session(token, kind).await {
            Ok(session) => {
                tracing::debug!("Got session: {:?}", session);
                // Always provide user info
                match load_authenticated_user(&session, &state.db).await {
                    Ok(user) => {
                        req = req.data(session.get_claims().clone());
                        req = req.data(session);
                        req = req.data(user);
                    }
                    Err(err) => {
                        tracing::error!("Error loading authenticated user: {:?}", &err);
                        return GraphQLResponse::from(err);
                    }
                }
            }
            Err(err) => {
                tracing::error!("Error getting session: {:?}", err);
            }
        }
    };

    // Set orchestrator
    req = req.data(get_orchestrator(&headers));

    // Check for x-refresh-token header
    let mut active_refresh = None;
    if let Some(refresh_token) = headers.get("x-refresh-token") {
        if let Ok(refresh_token) = refresh_token.to_str() {
            let session = state
                .session
                .lock()
                .await
                .get_refresh_session(refresh_token)
                .await;

            match session {
                Ok((refresh_session, current_token)) => {
                    tracing::debug!("Got refresh session: {:?}", &refresh_session);
                    req = req.data(refresh_session.clone());
                    active_refresh = Some((refresh_session, current_token));
                }
                Err(err) => {
                    tracing::error!("Error getting refresh session: {:?}", err);
                }
            }
        }
    }

    let mut resp = state.schema.execute(req).await;
    if resp.is_ok() {
        if let Some((refresh_session, _)) = active_refresh {
            let refreshed_data = showtimes_session::refresh_session(
                refresh_session.get_token(),
                &state.config.jwt.secret,
                state.config.jwt.get_expiration() as i64,
            );

            match refreshed_data {
                Ok(session_claims) => {
                    match state
                        .session
                        .lock()
                        .await
                        .set_refresh_session(
                            refresh_session.get_token(),
                            session_claims.get_token(),
                        )
                        .await
                    {
                        Err(err) => {
                            tracing::error!("Failed to save refresh session: {}", err);
                        }
                        Ok(_) => {
                            resp.http_headers.append(
                                "x-refreshed-token",
                                axum::http::HeaderValue::from_str(session_claims.get_token())
                                    .expect("Failed to serialize header value for refreshed token"),
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to refresh session: {}", e);
                }
            };
        }
    }

    resp.into()
}

/// Websocket handler for GraphQL
///
/// This Websocket handler use both `graphql-ws` and `graphql-transport-ws` protocols.
/// The handler will always need a token/session since all request is authenticated only.
pub async fn graphql_ws_handler(
    State(state): State<SharedShowtimesState>,
    headers: HeaderMap,
    protocol: GraphQLProtocol,
    websocket: WebSocketUpgrade,
) -> Response {
    websocket
        .protocols(ALL_WEBSOCKET_PROTOCOLS)
        .on_upgrade(move |stream| {
            GraphQLWebSocket::new(stream, state.schema.clone(), protocol)
                .on_connection_init(move |value| on_ws_init(Arc::clone(&state), headers, value))
                .serve()
        })
}

async fn on_ws_init(
    state: SharedShowtimesState,
    headers: HeaderMap,
    value: serde_json::Value,
) -> Result<GQLData, GQLError> {
    #[derive(serde::Deserialize)]
    struct Payload {
        token: String,
    }

    let mut data = GQLData::default();

    if let Ok(payload) = serde_json::from_value::<Payload>(value) {
        // If master key, format is MasterKey
        let session_kind = if payload.token == state.config.master_key {
            SessionKind::MasterKey
        } else if payload.token.starts_with("nsh_") {
            // Try parse API key, if fails then assume bearer
            showtimes_shared::APIKey::from_string(&payload.token)
                .map(|_| SessionKind::APIKey)
                .ok()
                .unwrap_or(SessionKind::Bearer)
        } else {
            SessionKind::Bearer
        };

        match state
            .session
            .lock()
            .await
            .get_session(payload.token, session_kind)
            .await
        {
            Ok(session) => {
                tracing::debug!("[WS] Got session (from payload): {:?}", session);
                data.insert(session.get_claims().clone());
                data.insert(session);
            }
            Err(err) => {
                tracing::error!("[WS] Error getting session (from payload): {:?}", err);
                return Err(GQLError::new(format!(
                    "Error validating token session: {}",
                    err
                )));
            }
        }
    } else if let Some((kind, token)) = get_token_or_bearer(&headers, &state.config) {
        match state.session.lock().await.get_session(token, kind).await {
            Ok(session) => {
                tracing::debug!("[WS] Got session (from header): {:?}", session);
                data.insert(session.get_claims().clone());
                data.insert(session);
            }
            Err(err) => {
                tracing::error!("[WS] Error getting session (from header): {:?}", err);
                return Err(GQLError::new(format!(
                    "Error validating token session: {}",
                    err
                )));
            }
        }
    } else {
        tracing::error!("[WS] No token found in payload or header");
        // Close/deny the connection
        return Err(GQLError::new("No token found in payload or header"));
    }

    data.insert(state.db.clone());
    data.insert(state.config.clone());

    let discord_client = DISCORD_CLIENT.get_or_init(|| {
        Arc::new(DiscordClient::new(
            state.config.discord.client_id.clone(),
            state.config.discord.client_secret.clone(),
        ))
    });

    data.insert(discord_client.clone());
    data.insert(state.meili.clone());
    data.insert(state.clickhouse.clone());
    data.insert(state.session.clone());
    data.insert(state.storage.clone());
    data.insert(state.anilist_provider.clone());
    if let Some(tmdb_provider) = state.tmdb_provider.as_ref() {
        data.insert(tmdb_provider.clone());
    }
    if let Some(vndb_provider) = state.vndb_provider.as_ref() {
        data.insert(vndb_provider.clone());
    }

    Ok(data)
}

async fn load_authenticated_user(
    session: &showtimes_session::ShowtimesUserSession,
    database: &DatabaseShared,
) -> Result<showtimes_db::m::User, GQLResponse> {
    let user_db = showtimes_db::UserHandler::new(database);

    let audience = session.get_claims().get_audience();

    let load_method = match audience {
        showtimes_session::ShowtimesAudience::User => {
            // load as ULID
            let user_id =
                showtimes_shared::ulid::Ulid::from_string(session.get_claims().get_metadata())
                    .map_err(|e| {
                        // make error
                        let gql_error = showtimes_gql_common::errors::GQLError::new(
                            e.to_string(),
                            GQLErrorCode::ParseUlidError,
                        )
                        .extend(|e| {
                            e.set("value", session.get_claims().get_metadata());
                            e.set("audience", audience.to_string());
                        })
                        .build();

                        error_to_gql_response(gql_error)
                    })?;

            user_db
                .find_by(showtimes_db::mongodb::bson::doc! {
                    "id": user_id.to_string(),
                })
                .await
                .map_err(|e| {
                    // make error
                    let gql_error = showtimes_gql_common::errors::GQLError::new(
                        e.to_string(),
                        GQLErrorCode::UserRequestFails,
                    )
                    .extend(|e| {
                        e.set("id", user_id.to_string());
                        e.set("audience", audience.to_string());
                        e.set("where", GQLDataLoaderWhere::UserLoaderId);
                    })
                    .build();

                    error_to_gql_response(gql_error)
                })?
        }
        showtimes_session::ShowtimesAudience::APIKey => {
            let api_key_raw = session.get_claims().get_metadata();
            let api_key = showtimes_shared::APIKey::try_from(api_key_raw).map_err(|e| {
                // make error
                let gql_error = showtimes_gql_common::errors::GQLError::new(
                    e.to_string(),
                    GQLErrorCode::ParseAPIKeyError,
                )
                .extend(|e| {
                    e.set("value", api_key_raw);
                    e.set("audience", audience.to_string());
                })
                .build();

                error_to_gql_response(gql_error)
            })?;

            user_db
                .find_by(showtimes_db::mongodb::bson::doc! {
                    "api_key.key": api_key.to_string(),
                })
                .await
                .map_err(|e| {
                    // make error
                    let gql_error = showtimes_gql_common::errors::GQLError::new(
                        e.to_string(),
                        GQLErrorCode::UserRequestFails,
                    )
                    .extend(|e| {
                        e.set("id", api_key.to_string());
                        e.set("audience", audience.to_string());
                        e.set("where", GQLDataLoaderWhere::UserLoaderAPIKey);
                    })
                    .build();

                    error_to_gql_response(gql_error)
                })?
        }
        showtimes_session::ShowtimesAudience::MasterKey => {
            let result = STUBBED_OWNER.get_or_init(|| {
                showtimes_db::m::User::stub_owner(session.get_claims().get_metadata())
            });

            Some(result.clone())
        }
        _ => {
            let gql_error = showtimes_gql_common::errors::GQLError::new(
                "Invalid audience type for this session",
                GQLErrorCode::UserInvalidAudience,
            )
            .extend(|e| {
                e.set("value", session.get_claims().get_metadata());
                e.set("audience", audience.to_string());
            })
            .build();

            return Err(error_to_gql_response(gql_error));
        }
    };

    match load_method {
        Some(user) => Ok(user),
        None => {
            let gql_error = showtimes_gql_common::errors::GQLError::new(
                "User not found",
                GQLErrorCode::UserNotFound,
            )
            .extend(|e| {
                e.set("id", session.get_claims().get_metadata());
                e.set("audience", audience.to_string());
            })
            .build();

            Err(error_to_gql_response(gql_error))
        }
    }
}

fn error_to_gql_response(error: GQLError) -> GQLResponse {
    let srv_error = GQLServerError {
        message: error.message,
        source: error.source,
        locations: vec![],
        path: vec![],
        extensions: error.extensions,
    };

    GQLResponse::from_errors(vec![srv_error])
}
