#![doc = include_str!("../README.md")]

use std::sync::Arc;

use axum::{response::IntoResponse, routing::get, Router};
use routes::graphql::{GRAPHQL_ROUTE, GRAPHQL_WS_ROUTE};
use serde_json::json;
use showtimes_fs::s3::S3FsCredentials;
use showtimes_shared::Config;
use tokio::{net::TcpListener, sync::Mutex};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod onion;
mod routes;
mod state;

#[tokio::main]
async fn main() {
    // Call our entrypoint function
    entrypoint().await.unwrap();
}

/// Actual main function
async fn entrypoint() -> anyhow::Result<()> {
    // get current working directory
    let cwd = std::env::current_dir().unwrap();

    // load the configuration file
    let config = match Config::async_load(cwd.join("config.toml")).await {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            anyhow::bail!("Failed to load configuration");
        }
    };

    let log_dir = match &config.log_directory {
        Some(log_dir) => {
            // Create the directory if not exists
            tokio::fs::create_dir_all(log_dir).await?;

            log_dir.clone()
        }
        None => {
            // Use cwd/logs
            let log_dir = cwd.join("logs");

            // Create the directory if not exists
            tokio::fs::create_dir_all(&log_dir).await?;

            log_dir.to_str().unwrap().to_string()
        }
    };
    let log_file = tracing_appender::rolling::daily(log_dir, "showtimes.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(log_file);

    let merged_env_trace = "showtimes=debug,showtimes_events=debug,tower_http=debug,axum::rejection=trace,async_graphql::graphql=debug,mongodb::connection=debug";

    // Initialize tracing logger
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .map(|filter| {
                    let split_filter = merged_env_trace.split(',').collect::<Vec<&str>>();
                    let directives = split_filter
                        .iter()
                        .fold(filter, |acc, &x| acc.add_directive(x.parse().unwrap()));
                    directives
                })
                .unwrap_or_else(|_| merged_env_trace.parse().unwrap()),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .init();

    let version = env!("CARGO_PKG_VERSION");
    tracing::info!("💭 Starting showtimes v{}", version);

    // Start loading database, storage, and other services
    tracing::info!("🔌 Loading services...");
    tracing::info!("🔌🔒 Loading session manager...");
    let session_manager =
        showtimes_session::manager::SessionManager::new(&config.database.redis, &config.jwt.secret)
            .await?;

    tracing::info!("🔌📅 Loading database...");
    let mongo_conn = showtimes_db::create_connection(&config.database.mongodb).await?;

    tracing::info!("🔌🪵 Loading ClickHouse events...");
    let clickhouse_conn = showtimes_events::SHClickHouse::new(
        &config.clickhouse.url,
        &config.clickhouse.username,
        config.clickhouse.password.as_deref(),
    )
    .await?;
    clickhouse_conn.create_tables().await?;

    // Initialize the filesystem
    tracing::info!("🔌📁 Loading filesystem...");
    let fs = match (&config.storages.s3, &config.storages.local) {
        (Some(s3), _) => {
            tracing::info!("🔌📁🚀 Using S3 filesystem");

            let s3_credentials = S3FsCredentials::new(&s3.access_key, &s3.secret_key);
            let s3_path_style = match s3.path_style {
                showtimes_shared::config::StorageS3PathStyle::Path => {
                    showtimes_fs::s3::S3PathStyle::Path
                }
                showtimes_shared::config::StorageS3PathStyle::Virtual => {
                    showtimes_fs::s3::S3PathStyle::VirtualHost
                }
            };

            let s3_bucket = showtimes_fs::s3::S3Fs::make_bucket(
                &s3.bucket,
                &s3.endpoint_url,
                &s3.region,
                Some(s3_path_style),
            );

            let s3_fs = showtimes_fs::s3::S3Fs::new(s3_bucket, s3_credentials);

            showtimes_fs::FsPool::S3Fs(s3_fs)
        }
        (_, Some(local)) => {
            tracing::info!("🔌📁🚀 Using local filesystem");
            let dir_path = std::path::PathBuf::from(&local.path);

            if !dir_path.exists() {
                anyhow::bail!("Local directory does not exist: {}", local.path);
            }

            let local_fs = showtimes_fs::local::LocalFs::new(dir_path);
            showtimes_fs::FsPool::LocalFs(local_fs)
        }
        _ => {
            anyhow::bail!("No storage configuration found");
        }
    };
    fs.init().await?;

    tracing::info!("🔌🔍 Loading search engine...");
    let meili =
        showtimes_search::create_connection(&config.meilisearch.url, &config.meilisearch.api_key)
            .await?;

    tracing::info!("🔌🐍 Loading external metadata services...");
    let anilist_provider = showtimes_metadata::AnilistProvider::new(true);
    let tmdb_provider = config
        .external
        .tmdb
        .as_ref()
        .map(|api_key| Arc::new(showtimes_metadata::TMDbProvider::new(api_key)));
    let vndb_provider = config
        .external
        .vndb
        .as_ref()
        .map(|api_key| Arc::new(showtimes_metadata::VndbProvider::new(api_key)));

    tracing::info!("🔌🚀 Loading GraphQL schema...");
    let schema = showtimes_gql::create_schema(&mongo_conn.db);

    tracing::info!("🔌 Initializing state...");
    let state = state::ShowtimesState {
        db: mongo_conn.db,
        storage: Arc::new(fs),
        meili,
        config: Arc::new(config.clone()),
        schema,
        session: Arc::new(Mutex::new(session_manager)),
        anilist_provider: Arc::new(Mutex::new(anilist_provider)),
        tmdb_provider,
        vndb_provider,
        clickhouse: Arc::new(clickhouse_conn),
    };

    tracing::info!("🚀 Starting server...");
    let app: Router = Router::new()
        .route("/", get(index))
        .route(
            GRAPHQL_ROUTE,
            get(routes::graphql::graphql_playground)
                .post(routes::graphql::graphql_handler)
                .layer(onion::GraphQLRequestLimit::new()),
        )
        .route(GRAPHQL_WS_ROUTE, get(routes::graphql::graphql_ws_handler))
        .route("/_/schema.graphql", get(routes::graphql::graphql_sdl))
        .route("/_/health", get(|| async { "OK" }))
        .route("/images/:id/:filename", get(routes::image::image_by_id))
        .route(
            "/images/:parent_id/:id/:filename",
            get(routes::image::image_by_id),
        )
        .route(
            "/oauth2/discord/authorize",
            get(routes::oauth2::oauth2_discord_authorize),
        )
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(vec![
                    // GET/POST for GraphQL stuff
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    // HEAD for additional metadata
                    axum::http::Method::HEAD,
                    // OPTIONS for CORS preflight
                    axum::http::Method::OPTIONS,
                    // CONNECT for other stuff
                    axum::http::Method::CONNECT,
                ])
                .allow_headers(tower_http::cors::Any),
        )
        .with_state(state);

    let listener = TcpListener::bind(format!(
        "{}:{}",
        config.host.clone().unwrap_or("127.0.0.1".to_string()),
        config.port.unwrap_or(5560),
    ))
    .await?;

    tracing::info!("🌍 Fast serving at http://{}", listener.local_addr()?);
    tracing::info!(
        "🌍 GraphQL playground: http://{}{}",
        listener.local_addr()?,
        GRAPHQL_ROUTE
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn index() -> impl IntoResponse {
    // json response saying "success": true and current version
    axum::Json(json!({ "success": true, "version": env!("CARGO_PKG_VERSION") }))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl-C, shutting down...");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, shutting down...");
        }
    }
}
