use async_graphql::dataloader::DataLoader;

use crate::data_loader::{DiscordIdLoad, UserDataLoader};

pub mod collaborations;
pub(crate) mod errors;
pub mod events;
pub mod prelude;
pub mod projects;
pub mod search;
pub mod servers;
pub mod stats;
pub mod users;

/// An orchestrator (or "on behalf-of") request information.
pub enum Orchestrator {
    /// A standalone request, means it's done by the current user
    Standalone,
    /// A request on behalf of a user via ID
    UserId(showtimes_shared::ulid::Ulid),
    /// A request on behalf of a user via Discord ID
    UserDiscord(String),
}

impl Orchestrator {
    /// Parse a `X-Orchestrator` header or standard string into an `Orchestrator`.
    /// By default, this will return [`Orchestrator::Standalone`].
    ///
    /// There will be no error if the header is missing or fails to parse.
    ///
    /// Sample header format:
    /// - `ID XXXXXXXXX` (with `XXXXXXXXX` being a ULID)
    /// - `Discord 123456789` (with `123456789` being a Discord ID)
    pub fn from_header<T: AsRef<str>>(header: Option<T>) -> Orchestrator {
        match header {
            Some(header) => {
                let header = header.as_ref();
                if header.starts_with("ID ") {
                    // Split ID <XXXXXXXXXXXX>, the parse as ULID
                    match header.get(3..) {
                        Some(id) => match showtimes_shared::ulid::Ulid::from_string(id) {
                            Ok(id) => Orchestrator::UserId(id),
                            Err(_) => Orchestrator::Standalone,
                        },
                        None => Orchestrator::Standalone,
                    }
                } else if header.starts_with("Discord ") {
                    match header.get(7..) {
                        Some(id) => Orchestrator::UserDiscord(id.to_string()),
                        None => Orchestrator::Standalone,
                    }
                } else {
                    Orchestrator::Standalone
                }
            }
            None => Orchestrator::Standalone,
        }
    }

    /// Request orchestrator information as a [`showtimes_db::m::User`].
    ///
    /// - If this is a [`Orchestrator::Standalone`], this will return `None`.
    /// - Otherwise, when the user is missing, this will return a stubbed user.
    pub(crate) async fn to_user(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Option<showtimes_db::m::User>> {
        let loader = ctx.data_unchecked::<DataLoader<UserDataLoader>>();

        match self {
            Orchestrator::Standalone => Ok(None),
            Orchestrator::UserId(id) => {
                let user = loader.load_one(*id).await?;
                Ok(Some(user.unwrap_or_else(|| {
                    showtimes_db::m::User::stub_with_id(*id)
                })))
            }
            Orchestrator::UserDiscord(id) => {
                let user = loader.load_one(DiscordIdLoad(id.clone())).await?;
                Ok(Some(user.unwrap_or_else(|| {
                    showtimes_db::m::User::stub_with_discord_id(id)
                })))
            }
        }
    }
}
