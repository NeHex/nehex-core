mod cache;
mod config;
mod error;
mod log_buffer;
mod routes;
mod startup;
mod state;
mod storage_local;

use std::{net::SocketAddr, sync::Arc};

use axum::{
    Router,
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use config::Settings;
use error::{AppError, AppResult};
use state::AppState;
use tokio::sync::RwLock;
use tower_http::{
    cors::{AllowHeaders, AllowMethods, Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, fmt, fmt::writer::MakeWriterExt};

const X_ROBOTS_TAG_HEADER_NAME: &str = "x-robots-tag";
const ADMIN_NOINDEX_HEADER_VALUE: &str = "noindex, nofollow, noarchive";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv().ok();
    init_tracing();

    if let Err(error) = run().await {
        eprintln!("{error}");
        return Err(anyhow::anyhow!(error.to_string()));
    }

    Ok(())
}

async fn run() -> AppResult<()> {
    let settings = Arc::new(Settings::load());
    let paths = Arc::new(state::ProjectPaths::discover());

    startup::check_admin_secret_safety(&settings)?;
    startup::ensure_admin_frontend(&settings, &paths)?;
    startup::wait_for_database_ready(&settings).await?;
    let db_pool = startup::create_db_pool(&settings).await?;
    startup::apply_startup_schema_maintenance(&settings, &db_pool).await;
    let runtime_cache = if settings.redis_enabled {
        Arc::new(cache::RuntimeCache::new_with_redis(
            settings.simple_cache_max_entries,
            cache::RedisCacheSettings {
                url: settings.redis_url.clone(),
                prefix: settings.redis_cache_prefix.clone(),
                retry_seconds: settings.redis_connect_retry_seconds,
                socket_connect_timeout_seconds: settings.redis_socket_connect_timeout,
                socket_timeout_seconds: settings.redis_socket_timeout,
            },
        ))
    } else {
        Arc::new(cache::RuntimeCache::new(settings.simple_cache_max_entries))
    };

    let state = AppState {
        settings: settings.clone(),
        db_pool,
        paths,
        admin_index_cache: Arc::new(RwLock::new(None)),
        admin_path_cache: Arc::new(RwLock::new(None)),
        runtime_cache,
        online_presence_hub: Arc::new(routes::ws_online::OnlinePresenceHub::new()),
    };

    let cors_layer = build_cors_layer(&settings);
    let admin_api_router =
        routes::admin_api::router().layer(middleware::from_fn(append_admin_noindex_header));

    let app = Router::new()
        .route("/health", get(routes::health::health))
        .route("/version", get(routes::health::version))
        .merge(routes::public_api::router())
        .merge(routes::storage_files::router())
        .merge(routes::ws_online::router())
        .nest("/admin-api", admin_api_router)
        .fallback(routes::admin_static::fallback_handler)
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer)
        .with_state(state);

    let bind_addr = SocketAddr::from(([0, 0, 0, 0], settings.app_port));
    info!(
        "starting {} {} on {bind_addr}",
        settings.app_name, settings.app_version
    );

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .map_err(|error| AppError::internal(format!("Failed to bind {bind_addr}: {error}")))?;

    axum::serve(listener, app)
        .await
        .map_err(|error| AppError::internal(format!("HTTP server exited unexpectedly: {error}")))?;

    Ok(())
}

async fn append_admin_noindex_header(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        HeaderName::from_static(X_ROBOTS_TAG_HEADER_NAME),
        HeaderValue::from_static(ADMIN_NOINDEX_HEADER_VALUE),
    );
    response
}

fn build_cors_layer(settings: &Settings) -> CorsLayer {
    let mut layer = CorsLayer::new();

    let has_wildcard = settings
        .cors_allow_origins
        .iter()
        .any(|origin| origin == "*");
    let is_dev_env = settings.is_dev_env();
    let allow_wildcard = has_wildcard && is_dev_env;

    if has_wildcard {
        if allow_wildcard {
            layer = layer.allow_origin(Any);
        } else {
            warn!(
                "CORS_ALLOW_ORIGINS contains '*' in non-dev APP_ENV={}, falling back to restrictive mode",
                settings.app_env
            );
        }
    } else {
        let origins = settings
            .cors_allow_origins
            .iter()
            .filter_map(|origin| origin.parse().ok())
            .collect::<Vec<http::HeaderValue>>();

        if origins.is_empty() {
            if is_dev_env {
                layer = layer.allow_origin(Any);
            } else {
                warn!(
                    "CORS_ALLOW_ORIGINS is empty or invalid in non-dev APP_ENV={}, keeping restrictive default",
                    settings.app_env
                );
            }
        } else {
            layer = layer.allow_origin(origins);
        }
    }

    if settings.cors_allow_credentials && !allow_wildcard {
        // tower-http rejects wildcard CORS headers/methods when credentials are enabled.
        layer = layer
            .allow_methods(AllowMethods::mirror_request())
            .allow_headers(AllowHeaders::mirror_request())
            .allow_credentials(true);
    } else {
        layer = layer.allow_methods(Any).allow_headers(Any);
    }

    layer
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let log_writer = std::io::stdout.and(log_buffer::make_writer());

    fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .with_writer(log_writer)
        .init();
}
