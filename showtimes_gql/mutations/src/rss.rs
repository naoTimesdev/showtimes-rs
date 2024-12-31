use std::sync::Arc;

use async_graphql::{dataloader::DataLoader, InputObject};
use showtimes_db::{mongodb::bson::doc, DatabaseShared};

use showtimes_gql_common::{
    data_loader::ServerDataLoader, errors::GQLError, GQLErrorCode, GQLErrorExt, UlidGQL,
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
}

/// The RSS display feed input object for updating an existing RSS feed
#[derive(InputObject)]
pub struct RSSFeedDisplayUpdateInputGQL {
    /// The message that will be send, maximum of 1500 characters
    ///
    /// This part cannot be removed entirely, if you don't want message leave it empty!
    #[graphql(validator(max_length = 1500))]
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
                    "User not allowed to create/delete projects",
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
            "User not allowed to create projects",
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
        .count_documents(doc! { "creator": srv.id.to_string() })
        .await
        .extend_error(GQLErrorCode::RSSFeedRequestFails, |e| {
            e.set("url", &input.url);
            e.set("server", srv.id.to_string());
            e.set("at", "count_query");
            input.dump_query(e);
        })?;

    let already_exist = rss_loader
        .find_by(doc! {
            "creator": srv.id.to_string(),
            "url": &input.url
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
    let feed_valid = showtimes_rss::test_feed_validity(&input.url)
        .await
        .map_err(|e| {
            GQLError::new(e.to_string(), GQLErrorCode::RSSFeedFetchError)
                .extend(|f| {
                    f.set("url", &input.url);
                    f.set("server", srv.id.to_string());
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
                    input.dump_query(f);
                })
                .build()
        })?;

    if !feed_valid {
        return GQLError::new("RSS feed is invalid", GQLErrorCode::RSSFeedInvalidFeed)
            .extend(|f| {
                f.set("url", &input.url);
                f.set("server", srv.id.to_string());
                input.dump_query(f);
            })
            .into();
    }

    // Guarantee that the URL is valid
    let url_parse = url::Url::parse(&input.url).expect("Failed to parse URL");

    let mut new_feed = showtimes_db::m::RSSFeed::new(url_parse, srv.id);

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
