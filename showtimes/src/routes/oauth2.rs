use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::state::SharedShowtimesState;

#[derive(Deserialize)]
pub struct DiscordAuthorizeQuery {
    redirect_url: String,
}

pub async fn oauth2_discord_authorize(
    State(state): State<SharedShowtimesState>,
    Query(DiscordAuthorizeQuery { redirect_url }): Query<DiscordAuthorizeQuery>,
) -> impl IntoResponse {
    let decoded_url = urlencoding::decode(&redirect_url).unwrap().to_string();

    let state_jack =
        showtimes_session::create_discord_session_state(&decoded_url, &state.jwt).unwrap();

    let scopes = ["identify", "email", "guilds", "guilds.members.read"];

    let query_params = [
        ("client_id", &state.config.discord.client_id),
        ("redirect_uri", &state.config.discord.redirect_url),
        ("response_type", &"code".to_string()),
        ("scope", &scopes.join(" ")),
        ("state", &state_jack),
        ("prompt", &"consent".to_string()),
    ];

    let query = query_params
        .iter()
        .map(|(key, value)| format!("{}={}", key, urlencoding::encode(value)))
        .collect::<Vec<String>>()
        .join("&");

    let discord_authorize = format!("https://discord.com/oauth2/authorize?{query}");

    (
        axum::http::StatusCode::FOUND,
        [(axum::http::header::LOCATION, discord_authorize)],
    )
}
