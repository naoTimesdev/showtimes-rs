use async_graphql::{dataloader::DataLoader, InputObject};
use showtimes_db::{mongodb::bson::doc, DatabaseShared};

use showtimes_gql_common::{
    data_loader::ServerDataLoader, errors::GQLError, GQLErrorCode, GQLErrorExt, UlidGQL,
};
use showtimes_gql_models::rss::RSSFeedGQL;

use crate::{IntegrationActionGQL, IntegrationInputGQL, IntegrationValidator};

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

pub async fn mutate_rss_feed_create(
    ctx: &async_graphql::Context<'_>,
    user: showtimes_db::m::User,
    input: RSSFeedCreateInputGQL,
) -> async_graphql::Result<RSSFeedGQL> {
    let db = ctx.data_unchecked::<DatabaseShared>().clone();

    let srv = check_permissions(ctx, *input.server, &user).await?;

    // Check if URL already exists
    let raw_loader = showtimes_db::RSSFeedHandler::new(&db);

    let already_exist = raw_loader
        .find_by(doc! {
            "creator": srv.id.to_string(),
            "url": &input.url
        })
        .await
        .extend_error(GQLErrorCode::RSSFeedRequestFails, |e| {
            e.set("url", &input.url);
            e.set("server", srv.id.to_string());
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

    raw_loader
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
