use std::sync::Arc;

use async_graphql::{dataloader::DataLoader, InputObject};
use showtimes_db::{mongodb::bson::doc, DatabaseShared};

use showtimes_gql_common::{
    data_loader::{RSSFeedLoader, ServerDataLoader},
    errors::GQLError,
    GQLErrorCode, GQLErrorExt, OkResponse, UlidGQL,
};
use showtimes_gql_models::rss::RSSFeedGQL;

use crate::{
    is_string_set, is_vec_set, IntegrationActionGQL, IntegrationInputGQL, IntegrationValidator,
};

/// The RSS feed input object for creating a new RSS feed
#[derive(InputObject)]
pub struct RSSFeedCreateInputGQL {
    /// The RSS URL
    #[graphql(validator(url))]
    url: String,
    /// The attached server
    server: UlidGQL,
    /// The list of integration to add, update, or remove
    #[graphql(validator(
        custom = "IntegrationValidator::with_limit(vec![IntegrationActionGQL::Add])"
    ))]
    integrations: Option<Vec<IntegrationInputGQL>>,
}

impl RSSFeedCreateInputGQL {
    /// Dump the input to the error context
    fn dump_query(&self, f_mut: &mut async_graphql::ErrorExtensionValues) {
        f_mut.set("url", &self.url);
        if let Some(integrations) = &self.integrations {
            f_mut.set(
                "integrations",
                integrations
                    .iter()
                    .map(|d| {
                        let mut f_new = async_graphql::indexmap::IndexMap::new();
                        d.dump_query(&mut f_new);
                        async_graphql::Value::Object(f_new)
                    })
                    .collect::<Vec<async_graphql::Value>>(),
            );
        }
    }
}

/// The RSS display embed feed input object for updating an existing RSS feed
#[derive(InputObject)]
pub struct RSSFeedEmbedDisplayUpdateInputGQL {
    /// The title of the RSS feed.
    title: Option<String>,
    /// The description of the RSS feed.
    description: Option<String>,
    /// The URL of the RSS feed.
    url: Option<String>,
    /// The thumbnail URL of the RSS feed.
    thumbnail: Option<String>,
    /// The image URL of the RSS feed.
    image: Option<String>,
    /// The footer of the RSS feed.
    footer: Option<String>,
    /// The footer image icon URL of the RSS feed.
    #[graphql(name = "footerImage")]
    footer_image: Option<String>,
    /// The author of the RSS feed.
    author: Option<String>,
    /// The author icon URL of the RSS feed.
    #[graphql(name = "authorImage")]
    author_image: Option<String>,
    /// The color of the RSS feed.
    color: Option<u32>,
    /// A boolean indicating whether the RSS feed is timestamped or not.
    timestamped: Option<bool>,
}

impl RSSFeedEmbedDisplayUpdateInputGQL {
    /// Check if any field is set
    fn is_any_set(&self) -> bool {
        is_string_set(&self.title)
            || is_string_set(&self.description)
            || is_string_set(&self.url)
            || is_string_set(&self.thumbnail)
            || is_string_set(&self.image)
            || is_string_set(&self.footer)
            || is_string_set(&self.footer_image)
            || is_string_set(&self.author)
            || is_string_set(&self.author_image)
            || self.color.is_some()
            || self.timestamped.is_some()
    }

    fn as_display(
        &self,
        original: &Option<showtimes_db::m::RSSFeedEmbedDisplay>,
    ) -> showtimes_db::m::RSSFeedEmbedDisplay {
        let timestamped = if let Some(embed) = original {
            self.timestamped.unwrap_or(embed.timestamped)
        } else {
            self.timestamped.unwrap_or(true)
        };

        showtimes_db::m::RSSFeedEmbedDisplay {
            title: self.title.clone(),
            description: self.description.clone(),
            url: self.url.clone(),
            thumbnail: self.thumbnail.clone(),
            image: self.image.clone(),
            footer: self.footer.clone(),
            footer_image: self.footer_image.clone(),
            author: self.author.clone(),
            author_image: self.author_image.clone(),
            color: self.color,
            timestamped,
        }
    }
}

/// The RSS display feed input object for updating an existing RSS feed
#[derive(InputObject)]
pub struct RSSFeedDisplayUpdateInputGQL {
    /// The message that will be send, maximum of 1500 characters
    ///
    /// This part cannot be removed entirely, if you don't want message leave it empty!
    #[graphql(validator(min_length = 1, max_length = 1500))]
    message: Option<String>,
    /// The embed display information of the RSS feed
    ///
    /// To remove the embed display information you should use `unsetEmbed` field.
    embed: Option<RSSFeedEmbedDisplayUpdateInputGQL>,
    /// Unset the embed display information of the RSS feed.
    ///
    /// This takes precedence over the `embed` field
    #[graphql(name = "unsetEmbed")]
    unset_embed: Option<bool>,
}

impl RSSFeedDisplayUpdateInputGQL {
    /// Check if any field is set
    fn is_any_set(&self) -> bool {
        let is_embed_set = self.embed.as_ref().map_or(false, |e| e.is_any_set());
        is_string_set(&self.message) || self.unset_embed.is_some() || is_embed_set
    }
}

/// The RSS feed input object for updating an existing RSS feed
#[derive(InputObject)]
pub struct RSSFeedUpdateInputGQL {
    /// The RSS URL
    #[graphql(validator(url))]
    url: Option<String>,
    /// The list of integration to add, update, or remove
    #[graphql(validator(custom = "IntegrationValidator::new()"))]
    integrations: Option<Vec<IntegrationInputGQL>>,
    /// The display information of the RSS feed
    display: Option<RSSFeedDisplayUpdateInputGQL>,
    /// Enable or disable the RSS feed
    enable: Option<bool>,
}

impl RSSFeedUpdateInputGQL {
    /// Dump the input to the error context
    fn dump_query(&self, f_mut: &mut async_graphql::ErrorExtensionValues) {
        if let Some(url) = &self.url {
            f_mut.set("url", url);
        }
        if let Some(integrations) = &self.integrations {
            f_mut.set(
                "integrations",
                integrations
                    .iter()
                    .map(|d| {
                        let mut f_new = async_graphql::indexmap::IndexMap::new();
                        d.dump_query(&mut f_new);
                        async_graphql::Value::Object(f_new)
                    })
                    .collect::<Vec<async_graphql::Value>>(),
            );
        }
    }

    /// Check if any field is set
    fn is_any_set(&self) -> bool {
        is_string_set(&self.url)
            || is_vec_set(&self.integrations)
            || self.display.as_ref().map_or(false, |d| d.is_any_set())
            || self.enable.is_some()
    }
}

async fn check_permissions(
    ctx: &async_graphql::Context<'_>,
    id: showtimes_shared::ulid::Ulid,
    user: &showtimes_db::m::User,
) -> async_graphql::Result<showtimes_db::m::Server> {
    let srv_loader = ctx.data_unchecked::<DataLoader<ServerDataLoader>>();

    let srv = srv_loader.load_one(id).await?.ok_or_else(|| {
        GQLError::new("Server not found", GQLErrorCode::ServerNotFound)
            .extend(|e| e.set("id", id.to_string()))
    })?;

    let find_user = srv.owners.iter().find(|o| o.id == user.id);

    match (find_user, user.kind) {
        (Some(user), showtimes_db::m::UserKind::User) => {
            // Check if we are allowed to adjust RSS info
            if user.privilege < showtimes_db::m::UserPrivilege::Admin {
                GQLError::new(
                    "User not allowed to create/modify/delete RSS feeds",
                    GQLErrorCode::UserInsufficientPrivilege,
                )
                .extend(|e| {
                    e.set("id", id.to_string());
                    e.set("current", user.privilege.to_string());
                    e.set(
                        "required",
                        showtimes_db::m::UserPrivilege::Admin.to_string(),
                    );
                    e.set("is_in_server", true);
                })
                .into()
            } else {
                Ok(srv)
            }
        }
        (None, showtimes_db::m::UserKind::User) => GQLError::new(
            "User not allowed to create/modify/delete RSS feeds",
            GQLErrorCode::UserInsufficientPrivilege,
        )
        .extend(|e| {
            e.set("id", id.to_string());
            e.set("is_in_server", false);
        })
        .into(),
        _ => {
            // Allow anyone to adjust RSS info
            Ok(srv)
        }
    }
}

fn has_valid_premium(premium_status: &[showtimes_db::m::ServerPremium]) -> bool {
    if premium_status.is_empty() {
        return true;
    }

    let current_time = chrono::Utc::now();

    premium_status.iter().any(|p| p.ends_at > current_time)
}

fn can_enable_rss(
    config: &Arc<showtimes_shared::Config>,
    rss_count: u64,
    premium_status: &[showtimes_db::m::ServerPremium],
) -> bool {
    let has_premium = has_valid_premium(premium_status);

    let limit: u64 = if has_premium {
        config.rss.premium_limit.unwrap_or(5)
    } else {
        config.rss.standard_limit.unwrap_or(2)
    }
    .into();

    rss_count < limit
}

pub async fn validate_rss_feed(
    url: &str,
    server_id: showtimes_shared::ulid::Ulid,
) -> Result<(), GQLError> {
    // Check if feed is Valid
    let feed_valid = showtimes_rss::test_feed_validity(url).await.map_err(|e| {
        GQLError::new(e.to_string(), GQLErrorCode::RSSFeedFetchError).extend(|f| {
            f.set("url", url);
            f.set("server", server_id.to_string());
            match e {
                showtimes_rss::RSSError::Feed(_) => {
                    f.set("kind", "feed_parse");
                }
                showtimes_rss::RSSError::InvalidUrl(_) => {
                    f.set("kind", "invalid_url");
                }
                showtimes_rss::RSSError::Reqwest(_) => {
                    f.set("kind", "http_request");
                }
            }
        })
    })?;

    if !feed_valid {
        Err(
            GQLError::new("RSS feed is invalid", GQLErrorCode::RSSFeedInvalidFeed).extend(|f| {
                f.set("url", url);
                f.set("server", server_id.to_string());
            }),
        )
    } else {
        Ok(())
    }
}

pub async fn mutate_rss_feed_create(
    ctx: &async_graphql::Context<'_>,
    user: showtimes_db::m::User,
    input: RSSFeedCreateInputGQL,
) -> async_graphql::Result<RSSFeedGQL> {
    let db = ctx.data_unchecked::<DatabaseShared>().clone();

    let current_time = chrono::Utc::now();
    let srv = check_permissions(ctx, *input.server, &user).await?;
    let premi_handler = showtimes_db::ServerPremiumHandler::new(&db);
    let current_time_bson = showtimes_db::mongodb::bson::DateTime::from_chrono(current_time);

    // Guarantee that the URL is valid
    let url_parsed = url::Url::parse(&input.url).map_err(|e| {
        GQLError::new(e.to_string(), GQLErrorCode::RSSFeedInvalidURL).extend(|f| {
            f.set("url", &input.url);
            f.set("server", srv.id.to_string());
        })
    })?;

    let premium_status = premi_handler
        .find_all_by(doc! {
            "target": srv.id.to_string(),
            "ends_at": { "$gte": current_time_bson }
        })
        .await
        .extend_error(GQLErrorCode::ServerPremiumRequestFails, |e| {
            e.set("server", srv.id.to_string());
            input.dump_query(e);
        })?;

    // Check if URL already exists
    let rss_loader = showtimes_db::RSSFeedHandler::new(&db);

    let rss_count = rss_loader
        .get_collection()
        .count_documents(doc! { "creator": srv.id.to_string(), "enabled": true })
        .await
        .extend_error(GQLErrorCode::RSSFeedRequestFails, |e| {
            e.set("url", &input.url);
            e.set("server", srv.id.to_string());
            e.set("at", "count_enabled_query");
            input.dump_query(e);
        })?;

    let already_exist = rss_loader
        .find_by(doc! {
            "creator": srv.id.to_string(),
            "url": url_parsed.as_str(),
        })
        .await
        .extend_error(GQLErrorCode::RSSFeedRequestFails, |e| {
            e.set("url", &input.url);
            e.set("server", srv.id.to_string());
            e.set("at", "verify_query");
            input.dump_query(e);
        })?;

    if already_exist.is_some() {
        return GQLError::new(
            "RSS feed already exists",
            GQLErrorCode::RSSFeedAlreadyExists,
        )
        .extend(|f| {
            f.set("url", &input.url);
            f.set("server", srv.id.to_string());
            input.dump_query(f);
        })
        .into();
    }

    // Check if feed is Valid
    validate_rss_feed(&input.url, srv.id).await.map_err(|e| {
        e.extend(|f| {
            input.dump_query(f);
        })
        .build()
    })?;

    let mut new_feed = showtimes_db::m::RSSFeed::new(url_parsed, srv.id);

    if let Some(integrations) = &input.integrations {
        let added_integration: Vec<showtimes_db::m::IntegrationId> = integrations
            .iter()
            .filter_map(|inter| match inter.action {
                IntegrationActionGQL::Add => Some(showtimes_db::m::IntegrationId::new(
                    inter.id.to_string(),
                    inter.kind.into(),
                )),
                _ => None,
            })
            .collect();
        new_feed.set_integrations(added_integration);
    }

    let config = ctx.data_unchecked::<Arc<showtimes_shared::Config>>();
    if !can_enable_rss(config, rss_count, &premium_status) {
        // Cannot enable more feeds
        new_feed.set_enabled(false);
    }

    rss_loader
        .save_direct(&mut new_feed, None)
        .await
        .extend_error(GQLErrorCode::RSSFeedCreateError, |f| {
            f.set("id", new_feed.id.to_string());
            f.set("url", &input.url);
            f.set("server", srv.id.to_string());
            input.dump_query(f);
        })?;

    let gql_feed = RSSFeedGQL::from(&new_feed);

    Ok(gql_feed)
}

pub async fn mutate_rss_feed_update(
    ctx: &async_graphql::Context<'_>,
    id: UlidGQL,
    user: showtimes_db::m::User,
    input: RSSFeedUpdateInputGQL,
) -> async_graphql::Result<RSSFeedGQL> {
    if !input.is_any_set() {
        return GQLError::new("No fields to update", GQLErrorCode::MissingModification).into();
    }

    let rss_loader = ctx.data_unchecked::<DataLoader<RSSFeedLoader>>();

    // Fetch feed
    let mut rss_feed = rss_loader.load_one(*id).await?.ok_or_else(|| {
        GQLError::new("RSS Feed not found", GQLErrorCode::RSSFeedNotFound)
            .extend(|e| e.set("id", id.to_string()))
    })?;

    let server = check_permissions(ctx, rss_feed.creator, &user).await?;

    if let Some(enabled) = input.enable {
        if enabled {
            let db = ctx.data_unchecked::<DatabaseShared>().clone();

            let current_time = chrono::Utc::now();
            let premi_handler = showtimes_db::ServerPremiumHandler::new(&db);
            let current_time_bson =
                showtimes_db::mongodb::bson::DateTime::from_chrono(current_time);

            let premium_status = premi_handler
                .find_all_by(doc! {
                    "target": server.id.to_string(),
                    "ends_at": { "$gte": current_time_bson }
                })
                .await
                .extend_error(GQLErrorCode::ServerPremiumRequestFails, |e| {
                    e.set("id", id.to_string());
                    e.set("server", server.id.to_string());
                    e.set("at", "premium_status_query");
                    input.dump_query(e);
                })?;

            // Check how much enabled feeds the server has
            let rss_count = rss_loader
                .loader()
                .get_inner()
                .get_collection()
                .count_documents(doc! { "creator": server.id.to_string(), "enabled": true })
                .await
                .extend_error(GQLErrorCode::RSSFeedRequestFails, |e| {
                    e.set("id", id.to_string());
                    e.set("server", server.id.to_string());
                    e.set("at", "count_enabled_query");
                    input.dump_query(e);
                })?;

            let config = ctx.data_unchecked::<Arc<showtimes_shared::Config>>();
            if !can_enable_rss(config, rss_count, &premium_status) {
                let has_premium = has_valid_premium(&premium_status);
                // Cannot enable more feeds
                return GQLError::new(
                    "Unable to enable more RSS feeds",
                    GQLErrorCode::RSSFeedLimitReached,
                )
                .extend(|f| {
                    f.set("id", id.to_string());
                    f.set("server", server.id.to_string());
                    f.set("rss_count", rss_count);
                    f.set("has_premium", has_premium);
                    f.set(
                        "limit",
                        if has_premium {
                            config.rss.premium_limit.unwrap_or(5)
                        } else {
                            config.rss.standard_limit.unwrap_or(2)
                        },
                    );
                    input.dump_query(f);
                })
                .into();
            }

            rss_feed.set_enabled(true);
        } else {
            rss_feed.set_enabled(false);
        }
    }

    if let Some(url) = &input.url {
        // Guarantee that the URL is valid
        let url_parsed = url::Url::parse(url).map_err(|e| {
            GQLError::new(e.to_string(), GQLErrorCode::RSSFeedInvalidURL).extend(|f| {
                f.set("url", url);
                f.set("server", server.id.to_string());
                input.dump_query(f);
            })
        })?;

        // Check if feed is Valid
        validate_rss_feed(url, server.id).await.map_err(|e| {
            e.extend(|f| {
                f.set("id", id.to_string());
                input.dump_query(f);
            })
            .build()
        })?;

        rss_feed.url = url_parsed;
    }

    if let Some(display) = &input.display {
        if let Some(true) = display.unset_embed {
            rss_feed.display.embed = None;
        } else if let Some(embed) = &display.embed {
            rss_feed.display.embed = Some(embed.as_display(&rss_feed.display.embed));
        }

        if let Some(message) = &display.message {
            rss_feed.display.message = Some(message.clone());
        }
    }

    for (idx, integration) in input
        .integrations
        .clone()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        match (integration.action, integration.original_id.clone()) {
            (IntegrationActionGQL::Add, _) => {
                // Check if the integration already exists
                let same_integration = rss_feed
                    .integrations
                    .iter()
                    .find(|i| i.id() == integration.id);

                if same_integration.is_none() {
                    rss_feed.add_integration(integration.into());
                }
            }
            (IntegrationActionGQL::Update, Some(original_id)) => {
                // Get olf integration
                let old_integration = server
                    .integrations
                    .iter()
                    .find(|i| i.id() == original_id)
                    .ok_or_else(|| {
                        GQLError::new("Integration not found", GQLErrorCode::IntegrationNotFound)
                            .extend(|e| {
                                e.set("id", &original_id);
                                e.set("feed", rss_feed.id.to_string());
                                e.set("action", "update");
                            })
                    })?;

                // Update the integration
                let new_integration = integration.into();
                rss_feed.remove_integration(old_integration);
                rss_feed.add_integration(new_integration);
            }
            (IntegrationActionGQL::Update, None) => {
                return GQLError::new(
                    "Integration original ID is required for update",
                    GQLErrorCode::IntegrationMissingOriginal,
                )
                .extend(|e| {
                    e.set("id", integration.id.to_string());
                    e.set("kind", integration.kind.to_string());
                    e.set("feed", rss_feed.id.to_string());
                    e.set("action", "update");
                    e.set("index", idx);
                })
                .into();
            }
            (IntegrationActionGQL::Remove, _) => {
                // Check if the integration exists
                let integration: showtimes_db::m::IntegrationId = integration.into();
                rss_feed.remove_integration(&integration);
            }
        }
    }

    rss_loader
        .loader()
        .get_inner()
        .save(&mut rss_feed, None)
        .await
        .extend_error(GQLErrorCode::RSSFeedUpdateError, |f_mut| {
            f_mut.set("id", rss_feed.id.to_string());
            f_mut.set("actor", user.id.to_string());
            input.dump_query(f_mut);
        })?;

    Ok(RSSFeedGQL::from(&rss_feed))
}

pub async fn mutate_rss_feed_delete(
    ctx: &async_graphql::Context<'_>,
    id: UlidGQL,
    user: showtimes_db::m::User,
) -> async_graphql::Result<OkResponse> {
    let rss_loader = ctx.data_unchecked::<DataLoader<RSSFeedLoader>>();

    // Fetch feed
    let rss_feed = rss_loader.load_one(*id).await?.ok_or_else(|| {
        GQLError::new("RSS Feed not found", GQLErrorCode::RSSFeedNotFound)
            .extend(|e| e.set("id", id.to_string()))
    })?;

    check_permissions(ctx, rss_feed.creator, &user).await?;

    rss_loader
        .loader()
        .get_inner()
        .delete(&rss_feed)
        .await
        .extend_error(GQLErrorCode::RSSFeedDeleteError, |f| {
            f.set("id", rss_feed.id.to_string());
            f.set("actor", user.id.to_string());
        })?;

    Ok(OkResponse::ok("RSS feed deleted"))
}
