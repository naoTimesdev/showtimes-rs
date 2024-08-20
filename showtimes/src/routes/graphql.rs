use std::sync::{Arc, OnceLock};

use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::State,
    http::HeaderMap,
    response::{Html, IntoResponse},
};
use showtimes_session::oauth2::discord::DiscordClient;

use crate::state::ShowtimesState;

pub const GRAPHQL_ROUTE: &str = "/graphql";
static DISCORD_CLIENT: OnceLock<Arc<DiscordClient>> = OnceLock::new();

pub async fn graphql_playground() -> impl IntoResponse {
    Html(showtimes_gql::playground_source(
        showtimes_gql::GraphQLPlaygroundConfig::new(GRAPHQL_ROUTE),
    ))
}

fn get_token_from_headers(headers: &HeaderMap) -> Option<String> {
    headers.get("Authorization").and_then(|value| {
        value.to_str().ok().and_then(|value| {
            if value.starts_with("Bearer ") {
                value.strip_prefix("Bearer ").map(|token| token.to_string())
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

    if let Some(token) = get_token_from_headers(&headers) {
        // Verify token
        if let Ok(claims) = showtimes_session::verify_session(&token, &state.config.jwt.secret) {
            req = req.data(claims.clone());
            req = req.data(showtimes_session::ShowtimesUserSession::new(token, claims));
        }
    };

    state.schema.execute(req).await.into()
}
