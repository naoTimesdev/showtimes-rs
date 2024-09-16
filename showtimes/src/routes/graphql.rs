use std::sync::{Arc, OnceLock};

use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::State,
    http::HeaderMap,
    response::{Html, IntoResponse},
};
use showtimes_gql::{graphiql_plugin_explorer, GraphiQLSource};
use showtimes_session::{manager::SessionKind, oauth2::discord::DiscordClient};
use showtimes_shared::Config;

use crate::state::ShowtimesState;

pub const GRAPHQL_ROUTE: &str = "/graphql";
static DISCORD_CLIENT: OnceLock<Arc<DiscordClient>> = OnceLock::new();

pub async fn graphql_playground() -> impl IntoResponse {
    let plugins = vec![graphiql_plugin_explorer()];
    let source = GraphiQLSource::build()
        .endpoint(GRAPHQL_ROUTE)
        .plugins(&plugins)
        .title("GraphiQL Playgronud")
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
