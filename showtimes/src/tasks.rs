use std::{collections::HashMap, sync::Arc};

use showtimes_db::{m::RSSFeed, mongodb::bson::doc};
use showtimes_rss::parse_feed;
use tokio_cron_scheduler::{JobScheduler, JobSchedulerError};

pub async fn shutdown_all_tasks(
    sched: &mut JobScheduler,
    jobs: &[uuid::Uuid],
) -> Result<(), JobSchedulerError> {
    tracing::info!("🔕 Removing {} tasks...", jobs.len());
    for job in jobs {
        sched.remove(job).await?;
    }

    tracing::info!("🔕 Shutting down task scheduler...");
    sched.shutdown().await?;
    Ok(())
}

async fn rss_single_task(
    feed: RSSFeed,
    state: Arc<crate::state::ShowtimesState>,
    handler: Arc<showtimes_db::RSSFeedHandler>,
) -> anyhow::Result<()> {
    tracing::debug!(
        "Spawning task for RSS feed: {} (for {})",
        &feed.url,
        feed.creator
    );

    let mut header_maps = reqwest::header::HeaderMap::new();
    if let Some(last_mod) = &feed.last_mod {
        header_maps.insert(
            reqwest::header::LAST_MODIFIED,
            reqwest::header::HeaderValue::from_str(last_mod)?,
        );
    }
    if let Some(etag) = &feed.etag {
        header_maps.insert(
            reqwest::header::ETAG,
            reqwest::header::HeaderValue::from_str(etag)?,
        );
    }

    let feed_data: showtimes_rss::FeedParsed<'_> =
        parse_feed(feed.url.to_string(), Some(header_maps)).await?;

    tracing::debug!(
        "Parsed a total of {} entries for RSS feed: {} (for {})",
        feed_data.entries.len(),
        &feed.url,
        feed.creator
    );
    let mut rss_manager = state.rss_manager.lock().await;
    let existing_keys = rss_manager.keys_exist(feed.id, &feed_data.entries).await?;

    // Drop so we don't lock it for other tasks.
    std::mem::drop(rss_manager);

    // Filter out existing keys so we don't push them again.
    let new_entries: Vec<showtimes_rss::FeedEntry> = feed_data
        .entries
        .iter()
        .filter(|x| {
            let key = showtimes_rss::manager::make_entry_key(x);
            !existing_keys.contains_key(&key)
        })
        .cloned()
        .collect::<Vec<_>>();

    tracing::debug!(
        "Pushing {} new entries for RSS feed: {} (for {})",
        new_entries.len(),
        &feed.url,
        feed.creator
    );

    if !new_entries.is_empty() {
        // Re-lock
        let mut rss_manager = state.rss_manager.lock().await;
        rss_manager.push_entries(feed.id, &new_entries).await?;
        // Drop
        std::mem::drop(rss_manager);

        // Publish events
        let rss_events: Vec<showtimes_events::m::RSSEvent> = new_entries
            .iter()
            .map(|x| showtimes_events::m::RSSEvent::from_entry(feed.id, feed.creator, x))
            .collect();

        state.clickhouse.create_rss_many_async(rss_events);
    }

    // Update etag and last modified
    let mut cloned_feed = feed.clone();
    let mut changed = false;
    if let Some(last_mod) = feed_data.last_modified {
        cloned_feed.last_mod = Some(last_mod);
        changed = true;
    }
    if let Some(etag) = feed_data.etag {
        cloned_feed.etag = Some(etag);
        changed = true;
    }

    if changed {
        tracing::debug!("Updating RSS feed: {} (for {})", &feed.url, feed.creator);
        handler.save(&mut cloned_feed, None).await?;
    }

    Ok(())
}

async fn tasks_rss_common(
    state: Arc<crate::state::ShowtimesState>,
    is_premium: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let rss_limit = if is_premium {
        state.config.rss.premium_limit.unwrap_or(5)
    } else {
        state.config.rss.standard_limit.unwrap_or(2)
    };

    let current_time = chrono::Utc::now();
    let handler_rss = Arc::new(showtimes_db::RSSFeedHandler::new(&state.db));
    let handler_premium = showtimes_db::ServerPremiumHandler::new(&state.db);

    let current_time_bson = showtimes_db::mongodb::bson::DateTime::from_chrono(current_time);

    let premium_server = handler_premium
        .find_all_by(doc! {
            // Each premium ends at specific time, ensure we only get the one that is active right now.
            "ends_at": { "$gte": current_time_bson }
        })
        .await?;

    let premium_server_ids: Vec<String> = premium_server
        .iter()
        .map(|x| x.target.to_string())
        .collect();

    let query_feeds = if premium_server_ids.is_empty() && !is_premium {
        doc! {
            "enabled": true,
        }
    } else if !is_premium {
        doc! {
            "enabled": true,
            "creator": { "$nin": premium_server_ids },
        }
    } else {
        doc! {
            "enabled": true,
            "creator": { "$in": premium_server_ids },
        }
    };

    tracing::debug!("Running RSS fetch with query: {:?}", &query_feeds);
    let result_feeds = handler_rss.find_all_by(query_feeds).await?;

    // Map to HashMap for each "server"/"creator"
    let mut mapped_feeds: HashMap<showtimes_shared::ulid::Ulid, Vec<RSSFeed>> = HashMap::new();
    for feed in result_feeds {
        let map = mapped_feeds.entry(feed.creator).or_default();
        map.push(feed);
    }

    // Ensure each server has at most standard_limit feeds
    for (_, feeds) in mapped_feeds.iter_mut() {
        feeds.sort_by_key(|x| x.created);
        feeds.truncate(rss_limit.try_into()?);

        for feed in feeds.iter() {
            let cloned_state = Arc::clone(&state);
            let cloned_handler = Arc::clone(&handler_rss);
            let feed = feed.clone();
            tokio::task::spawn_local(async move {
                let feed_url = feed.url.clone();
                let creator = feed.creator.to_string();
                let res = rss_single_task(feed, cloned_state, cloned_handler).await;
                match res {
                    Ok(_) => {
                        tracing::debug!(
                            "Finished task for RSS feed: {} (for {})",
                            &feed_url,
                            &creator
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "Error processing RSS feed `{}`: {} (is_premium? {}, for {})",
                            &feed_url,
                            e,
                            is_premium,
                            &creator
                        );
                    }
                }
            });
        }
    }

    Ok(())
}

pub async fn tasks_rss_standard(
    state: Arc<crate::state::ShowtimesState>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!("Running tasks_rss_standard");
    tasks_rss_common(state, false).await
}

pub async fn tasks_rss_premium(
    state: Arc<crate::state::ShowtimesState>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!("Running tasks_rss_premium");
    tasks_rss_common(state, true).await
}
