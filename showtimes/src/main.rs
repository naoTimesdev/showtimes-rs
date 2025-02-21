#![doc = include_str!("../README.md")]

use std::{sync::Arc, time::Duration};

use axum::{Router, response::IntoResponse, routing::get};
use routes::graphql::{GRAPHQL_ROUTE, GRAPHQL_WS_ROUTE};
use serde_json::json;
use showtimes_fs::s3::S3FsCredentials;
use showtimes_shared::Config;
use tasks::{shutdown_all_tasks, tasks_rss_premium, tasks_rss_standard};
// use tasks::{spawn_with, RSSTasks};
use tokio::{net::TcpListener, sync::Mutex};
use tokio_cron_scheduler::{Job, JobScheduler};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod onion;
mod routes;
mod state;
mod tasks;

const ASSET_ICON: &[u8] = include_bytes!("../assets/icon.ico");

#[tokio::main]
async fn main() {
    // Call our entrypoint function
    entrypoint().await.unwrap();
}

/// Actual main function
async fn entrypoint() -> anyhow::Result<()> {
    // get current working directory
    let cwd = std::env::current_dir().unwrap();

    let commit_hash = env!("GIT_COMMIT");
    let commit_short = &commit_hash[..8];

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
    tracing::info!("💭 Starting showtimes v{}+g{}", version, commit_short);

    // Verify config
    tracing::info!("🔍 Verifying configuration...");
    match config.verify() {
        Ok(_) => {
            tracing::info!("🔍✅ Configuration verified");
        }
        Err(e) => {
            tracing::error!("🔍⚠️ Configuration verification failed: {}", e);
            anyhow::bail!("Configuration verification failed");
        }
    }

    // Initialize JWT session
    tracing::info!("🔌🔑 Initializing JWT keys...");
    let jwt_variant = match config.jwt.variant.unwrap_or_default() {
        showtimes_shared::config::JWTSHAMode::SHA256 => showtimes_session::signer::SHALevel::SHA256,
        showtimes_shared::config::JWTSHAMode::SHA384 => showtimes_session::signer::SHALevel::SHA384,
        showtimes_shared::config::JWTSHAMode::SHA512 => showtimes_session::signer::SHALevel::SHA512,
    };
    let jwt_key = match &config.jwt.mode {
        showtimes_shared::config::JWTMode::HMAC => {
            let secret = config.jwt.secret.clone().expect("JWT secret is missing");
            let hmac_alg = showtimes_session::signer::HmacAlgorithm::new(jwt_variant, secret);
            showtimes_session::signer::Signer::Hmac(hmac_alg)
        }
        showtimes_shared::config::JWTMode::RSA
        | showtimes_shared::config::JWTMode::RSAPSS
        | showtimes_shared::config::JWTMode::ECDSA
        | showtimes_shared::config::JWTMode::EDDSA
        | showtimes_shared::config::JWTMode::ES256K1 => {
            // read the public key
            let public_key = tokio::fs::read(
                config
                    .jwt
                    .public_key
                    .clone()
                    .expect("JWT public key is missing"),
            )
            .await?;
            let private_key = tokio::fs::read(
                config
                    .jwt
                    .private_key
                    .clone()
                    .expect("JWT private key is missing"),
            )
            .await?;

            match config.jwt.mode {
                showtimes_shared::config::JWTMode::ECDSA => {
                    let ecdsa_alg = showtimes_session::signer::EcdsaAlgorithm::new_pem(
                        jwt_variant,
                        private_key,
                        public_key,
                    )?;
                    showtimes_session::signer::Signer::Ecdsa(ecdsa_alg)
                }
                showtimes_shared::config::JWTMode::ES256K1 => {
                    let secp256k1_alg = showtimes_session::signer::Secp256k1Algorithm::new_pem(
                        private_key,
                        public_key,
                    )?;
                    showtimes_session::signer::Signer::Secp256k1(secp256k1_alg)
                }
                showtimes_shared::config::JWTMode::EDDSA => {
                    let eddsa_alg = showtimes_session::signer::Ed25519Algorithm::new_pem(
                        private_key,
                        public_key,
                    )?;
                    showtimes_session::signer::Signer::Ed25519(eddsa_alg)
                }
                showtimes_shared::config::JWTMode::RSA => {
                    let rsa_alg = showtimes_session::signer::RsaAlgorithm::new_pem(
                        jwt_variant,
                        private_key,
                        public_key,
                    )?;
                    showtimes_session::signer::Signer::Rsa(rsa_alg)
                }
                showtimes_shared::config::JWTMode::RSAPSS => {
                    let rsa_pss_alg = showtimes_session::signer::RsaPssAlgorithm::new_pem(
                        jwt_variant,
                        private_key,
                        public_key,
                    )?;
                    showtimes_session::signer::Signer::RsaPss(rsa_pss_alg)
                }
                _ => {
                    unreachable!("This JWT branch should not be reachable");
                }
            }
        }
    };

    // Test JWT encode/deocde process
    tracing::info!("🔌🔑🔒 Testing encoding key...");
    match showtimes_session::signer::test_encode(&jwt_key) {
        Ok(key) => {
            tracing::info!("🔌🔑🔒✅ Encoding key tested and successful");
            tracing::info!("🔌🔑🔓 Testing token decoding...");
            match showtimes_session::signer::test_decode(&key, &jwt_key) {
                Ok(_) => {
                    tracing::info!("🔌🔑🔓✅ Decoding key tested and successful");
                }
                Err(e) => {
                    tracing::error!("🔌🔑🔓⚠️ Decoding key test failed: {}", e);
                    anyhow::bail!("Decoding key test failed");
                }
            }
        }
        Err(e) => {
            tracing::error!("🔌🔑🔒⚠️ Encoding key test failed: {}", e);
            anyhow::bail!("Encoding key test failed");
        }
    }

    let arc_jwt = Arc::new(jwt_key);

    // Start loading database, storage, and other services
    tracing::info!("🔌 Loading services...");
    tracing::info!("🔌📒 Loading Redis cache...");
    let redis_conn = Arc::new(redis::Client::open(config.database.redis.clone())?);
    tracing::info!("🔌🔒 Loading session manager...");
    let session_manager =
        showtimes_session::manager::SessionManager::new(&redis_conn, &arc_jwt).await?;
    tracing::info!("🔌📰 Loading RSS manager...");
    let rss_manager = showtimes_rss::manager::RSSManager::new(&redis_conn).await?;

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
            )?;

            let s3_fs = showtimes_fs::s3::S3Fs::new(s3_bucket, s3_credentials)?;

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
    let schema = crate::routes::graphql::create_schema(&mongo_conn.db);

    tracing::info!("🔌 Initializing state...");
    let state = state::ShowtimesState {
        db: mongo_conn.db,
        storage: Arc::new(fs),
        meili,
        config: Arc::new(config.clone()),
        schema,
        session: Arc::new(Mutex::new(session_manager)),
        jwt: arc_jwt,
        rss_manager: Arc::new(Mutex::new(rss_manager)),
        anilist_provider: Arc::new(Mutex::new(anilist_provider)),
        tmdb_provider,
        vndb_provider,
        clickhouse: Arc::new(clickhouse_conn),
    };
    let shared_state = Arc::new(state);

    tracing::info!("🚀 Starting server...");
    let app = Router::new()
        .route("/", get(index))
        .route("/favicon.ico", get(index_favicons))
        .route(
            GRAPHQL_ROUTE,
            get(routes::graphql::graphql_playground)
                .post(routes::graphql::graphql_handler)
                .layer(onion::GraphQLRequestLimit::new()),
        )
        .route(GRAPHQL_WS_ROUTE, get(routes::graphql::graphql_ws_handler))
        .route("/_/schema.graphql", get(routes::graphql::graphql_sdl))
        .route("/_/health", get(|| async { "OK" }))
        .route("/images/{id}/{filename}", get(routes::image::image_by_id))
        .route(
            "/images/{parent_id}/{id}/{filename}",
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
        .with_state(Arc::clone(&shared_state));

    tracing::info!("🌐 Creating HTTP listener...");
    let listener = TcpListener::bind(format!(
        "{}:{}",
        config.host.clone().unwrap_or("127.0.0.1".to_string()),
        config.port.unwrap_or(5560),
    ))
    .await?;

    // Start tasks
    tracing::info!("⚡ Preparing task scheduler...");
    let mut active_jobs: Vec<uuid::Uuid> = Vec::new();
    let mut scheduler = JobScheduler::new().await?;
    if config.rss.enabled {
        let standard_dur = Duration::from_secs(config.rss.standard.unwrap_or(60 * 5).into());
        let premium_dur = Duration::from_secs(config.rss.premium.unwrap_or(60 * 2).into());

        let cloned_state = Arc::clone(&shared_state);
        let job_rss_standard = Job::new_repeated_async(standard_dur, move |_uuid, _lock| {
            Box::pin({
                let value = cloned_state.clone();
                async move {
                    match tasks_rss_standard(value).await {
                        Ok(_) => (),
                        Err(e) => {
                            tracing::error!("RSS standard task failed: {}", e);
                        }
                    }
                }
            })
        })?;

        let cloned_state = Arc::clone(&shared_state);
        let job_rss_premium = Job::new_repeated_async(premium_dur, move |_uuid, _lock| {
            Box::pin({
                let value = cloned_state.clone();
                async move {
                    match tasks_rss_premium(value).await {
                        Ok(_) => (),
                        Err(e) => {
                            tracing::error!("RSS premium task failed: {}", e);
                        }
                    }
                }
            })
        })?;

        let rss_std_uuid = scheduler.add(job_rss_standard).await?;
        let rss_premi_uuid = scheduler.add(job_rss_premium).await?;

        active_jobs.push(rss_std_uuid);
        active_jobs.push(rss_premi_uuid);
    }

    tracing::info!("⚡ Starting task scheduler...");
    scheduler.start().await?;

    // Spawn the axum server
    let local_addr = listener.local_addr()?;
    tracing::info!("🌍 Fast serving at http://{}", local_addr);
    tracing::info!(
        "🌍 GraphQL playground: http://{}{}",
        local_addr,
        GRAPHQL_ROUTE
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    // Stop tasks
    tracing::info!("🔕 Shutting down task scheduler...");
    shutdown_all_tasks(&mut scheduler, &active_jobs).await?;

    Ok(())
}

async fn index() -> impl IntoResponse {
    // json response saying "success": true and current version
    axum::Json(
        json!({ "success": true, "version": env!("CARGO_PKG_VERSION"), "commit": env!("GIT_COMMIT") }),
    )
}

async fn index_favicons() -> impl IntoResponse {
    let etag = format!("sh-favicons-{}", env!("CARGO_PKG_VERSION"));

    axum::http::Response::builder()
        .status(axum::http::StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, "image/x-icon")
        .header(
            axum::http::header::CACHE_CONTROL,
            "public, max-age=604800, immutable",
        )
        .header(axum::http::header::ETAG, etag)
        .body(axum::body::Body::from(ASSET_ICON))
        .unwrap()
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
