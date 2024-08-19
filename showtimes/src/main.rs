use std::sync::Arc;

use axum::{response::IntoResponse, routing::get, Router};
use serde_json::json;
use showtimes_fs::s3::{S3FsCredentialsProvider, S3FsRegionProvider};
use showtimes_shared::Config;
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    // Initialize tracing logger
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "showtimes=debug,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .init();

    let version = env!("CARGO_PKG_VERSION");
    tracing::info!("💭 Starting showtimes v{}", version);

    // Start loading database, storage, and other services
    tracing::info!("🔌 Loading services...");
    tracing::info!("🔌📅 Loading database...");
    let mongo_conn = showtimes_db::create_connection(&config.database.mongodb).await?;

    // Initialize the filesystem
    tracing::info!("🔌📁 Loading filesystem...");
    let fs = match (&config.storages.s3, &config.storages.local) {
        (Some(s3), _) => {
            tracing::info!("🔌📁🚀 Using S3 filesystem");
            let credentials = S3FsCredentialsProvider::new(&s3.access_key, &s3.secret_key);
            let region_info = match &s3.endpoint_url {
                Some(endpoint) => S3FsRegionProvider::new(&s3.region, Some(endpoint)),
                None => S3FsRegionProvider::new(&s3.region, None),
            };

            let s3_fs = showtimes_fs::s3::S3Fs::new(&s3.bucket, credentials, region_info).await;

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

    tracing::info!("🔌 Initializing state...");
    let state = state::ShowtimesState {
        db: mongo_conn.db,
        storage: Arc::new(fs),
        meili,
        config: Arc::new(config.clone()),
    };

    tracing::info!("🚀 Starting server...");
    let app: Router = Router::new()
        .route("/", get(index))
        .route("/_/health", get(|| async { "OK" }))
        .route("/images/:id/:filename", get(routes::image::image_by_id))
        .route(
            "/images/:parent_id/:id/:filename",
            get(routes::image::image_by_id),
        )
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::new().allow_origin(tower_http::cors::Any))
        .with_state(state);

    let listener = TcpListener::bind(format!(
        "{}:{}",
        config.host.clone().unwrap_or("127.0.0.1".to_string()),
        config.port.unwrap_or(5560)
    ))
    .await?;
    tracing::info!("🌍 Fast serving at http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index() -> impl IntoResponse {
    // json response saying "success": true and current version
    axum::Json(json!({ "success": true, "version": env!("CARGO_PKG_VERSION") }))
}
