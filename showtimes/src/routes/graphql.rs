use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use async_graphql_axum::{GraphQLProtocol, GraphQLRequest, GraphQLResponse, GraphQLWebSocket};
use axum::{
    extract::{State, WebSocketUpgrade},
    http::HeaderMap,
    response::{Html, IntoResponse, Response},
};
use showtimes_gql::{
    AltairConfigOptions, AltairSettingsState, AltairSource, AltairWindowOptions, Data as GQLData,
    Error as GQLError, ALL_WEBSOCKET_PROTOCOLS,
};
use showtimes_session::{manager::SessionKind, oauth2::discord::DiscordClient};
use showtimes_shared::Config;

use crate::state::ShowtimesState;

pub const GRAPHQL_ROUTE: &str = "/graphql";
pub const GRAPHQL_WS_ROUTE: &str = "/graphql/ws";
static DISCORD_CLIENT: OnceLock<Arc<DiscordClient>> = OnceLock::new();

const INITIAL_QUERY: &str = r#"# Welcome to Showtimes API
# 
# Showtimes is a management tools for a group to manage and track their
# traanslations, releases, and more for many multimedia projects focused on Japanese media.
#
# The following is a playground to test your queries. This playground is a fully
# functional GraphQL IDE with the ability to save your queries.
#
# Type queries into this side of the screen, and you will see intelligent
# typeaheads aware of the current GraphQL type schema and live syntax and
# validation errors highlighted within the text.
#
# GraphQL queries typically start with a "{" character. Lines that start
# with a # are ignored.
#
# An example GraphQL query might look like:
#
#     {
#       field(arg: "value") {
#         subField
#       }
#     }
#
#

query getCurrentUser {
    current {
        user {
            id
            username
            apiKey
            avatar {
                url
            }
        }
        token
    }
}
"#;

pub async fn graphql_playground() -> impl IntoResponse {
    let default_headers = HashMap::from([(
        "Authorization".to_string(),
        "Token nsh_your-api-token".to_string(),
    )]);

    let source = AltairSource::build()
        .options(AltairConfigOptions {
            window_options: Some(AltairWindowOptions {
                endpoint_url: Some(GRAPHQL_ROUTE.to_string()),
                subscriptions_endpoint: Some(GRAPHQL_WS_ROUTE.to_string()),
                initial_query: Some(INITIAL_QUERY.to_string()),
                initial_name: Some("Showtimes API".to_string()),
                initial_headers: default_headers,
                ..Default::default()
            }),
            disable_account: Some(true),
            instance_storage_namespace: Some("altair_showtimes_".to_string()),
            initial_settings: Some(AltairSettingsState {
                tab_size: Some(4),
                plugin_list: vec!["altair-graphql-plugin-graphql-explorer".to_string()],
                schema_reload_on_start: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        })
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

pub async fn graphql_ws_handler(
    State(state): State<ShowtimesState>,
    protocol: GraphQLProtocol,
    websocket: WebSocketUpgrade,
) -> Response {
    websocket
        .protocols(ALL_WEBSOCKET_PROTOCOLS)
        .on_upgrade(move |stream| {
            GraphQLWebSocket::new(stream, state.schema.clone(), protocol)
                .on_connection_init(move |value| on_ws_init(state.clone(), value))
                .serve()
        })
}

async fn on_ws_init(state: ShowtimesState, value: serde_json::Value) -> Result<GQLData, GQLError> {
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
                .unwrap_or_else(|| SessionKind::Bearer)
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
                tracing::debug!("[WS] Got session: {:?}", session);
                data.insert(session.get_claims().clone());
                data.insert(session);
            }
            Err(err) => {
                tracing::error!("[WS] Error getting session: {:?}", err);
            }
        }
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
