use std::sync::{Arc, OnceLock};

use async_graphql_axum::{GraphQLProtocol, GraphQLRequest, GraphQLResponse, GraphQLWebSocket};
use axum::{
    extract::{State, WebSocketUpgrade},
    http::HeaderMap,
    response::{Html, IntoResponse, Response},
};
use showtimes_gql::{
    graphiql_plugin_explorer, Data as GQLData, Error as GQLError, GraphiQLSource,
    ALL_WEBSOCKET_PROTOCOLS,
};
use showtimes_session::{manager::SessionKind, oauth2::discord::DiscordClient};
use showtimes_shared::Config;

use crate::state::ShowtimesState;

pub const GRAPHQL_ROUTE: &str = "/graphql";
pub const GRAPHQL_WS_ROUTE: &str = "/graphql/ws";
static DISCORD_CLIENT: OnceLock<Arc<DiscordClient>> = OnceLock::new();

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

/// The main GraphQL handler
///
/// This handler will handle all GraphQL requests, it will also handle the authentication
pub async fn graphql_handler(
    State(state): State<ShowtimesState>,
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
                req = req.data(session.get_claims().clone());
                req = req.data(session);
            }
            Err(err) => {
                tracing::error!("Error getting session: {:?}", err);
            }
        }
    };

    state.schema.execute(req).await.into()
}

/// Websocket handler for GraphQL
///
/// This Websocket handler use both `graphql-ws` and `graphql-transport-ws` protocols.
/// The handler will always need a token/session since all request is authenticated only.
pub async fn graphql_ws_handler(
    State(state): State<ShowtimesState>,
    headers: HeaderMap,
    protocol: GraphQLProtocol,
    websocket: WebSocketUpgrade,
) -> Response {
    websocket
        .protocols(ALL_WEBSOCKET_PROTOCOLS)
        .on_upgrade(move |stream| {
            GraphQLWebSocket::new(stream, state.schema.clone(), protocol)
                .on_connection_init(move |value| on_ws_init(state.clone(), headers, value))
                .serve()
        })
}

async fn on_ws_init(
    state: ShowtimesState,
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
