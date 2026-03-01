mod auth;
mod config;
mod db;
mod oauth;
mod routes;
mod state;
mod tick;
mod ws;

use std::sync::Arc;
use axum::http::{header, HeaderName, HeaderValue, Method};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

use config::ServerConfig;
use state::AppState;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("gt_server=info,tower_http=info")),
        )
        .init();

    info!("GlobalTelco server starting...");

    // Load configuration from environment
    let config = ServerConfig::from_env();

    info!("Host: {}", config.host);
    info!("Port: {}", config.port);
    info!(
        "Database: {}",
        if config.database_url.is_some() {
            "configured"
        } else {
            "in-memory (no DATABASE_URL set)"
        }
    );
    if let Some(ref tile_dir) = config.tile_dir {
        info!("Tile directory: {}", tile_dir);
    } else {
        info!("Tile serving: disabled (no TILE_DIR set)");
    }

    // Connect to PostgreSQL if DATABASE_URL is set
    #[cfg(feature = "postgres")]
    let database = if let Some(ref url) = config.database_url {
        match db::Database::connect(url).await {
            Ok(db) => {
                info!("Connected to PostgreSQL");
                if let Err(e) = db.run_migrations().await {
                    tracing::error!("Failed to run migrations: {e}");
                }
                Some(db)
            }
            Err(e) => {
                tracing::error!("Failed to connect to PostgreSQL: {e}");
                None
            }
        }
    } else {
        None
    };

    #[cfg(not(feature = "postgres"))]
    let database: Option<db::Database> = None;

    // Create shared state
    let state = Arc::new(
        AppState::new(config.auth.clone(), database)
            .with_oauth(config.oauth.clone())
            .with_cf_reset_url(config.cf_reset_worker_url.clone()),
    );

    // Create a default world for local testing
    let default_world_id = state
        .create_world(
            config.default_world_name.clone(),
            gt_common::types::WorldConfig::default(),
            config.default_max_players,
        )
        .await;

    info!("Created default world: {}", default_world_id);

    // Start tick loop for the default world
    if let Some(world) = state.get_world(&default_world_id).await {
        #[cfg(feature = "postgres")]
        tick::spawn_world_tick_loop(world, state.db.clone());
        #[cfg(not(feature = "postgres"))]
        tick::spawn_world_tick_loop(world);
    }

    // CORS — restrict in production when CORS_ORIGIN is set
    let cors = if let Ok(origin) = std::env::var("CORS_ORIGIN") {
        info!("CORS restricted to origin: {}", origin);
        CorsLayer::new()
            .allow_origin(origin.parse::<HeaderValue>().expect("Invalid CORS_ORIGIN value"))
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
            .allow_headers([
                header::AUTHORIZATION,
                header::CONTENT_TYPE,
                header::UPGRADE,
                header::CONNECTION,
                HeaderName::from_static("sec-websocket-key"),
                HeaderName::from_static("sec-websocket-version"),
                HeaderName::from_static("sec-websocket-protocol"),
                HeaderName::from_static("x-admin-key"),
            ])
    } else {
        info!("CORS open (no CORS_ORIGIN set — dev mode)");
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    };

    // Build router
    let app = routes::create_router(Arc::clone(&state), config.tile_dir.clone())
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let bind_addr = config.bind_addr();
    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    info!("Listening on http://{}", bind_addr);
    info!("WebSocket endpoint: ws://{}/ws", bind_addr);
    info!("REST API: http://{}/api", bind_addr);
    info!("Health check: http://{}/health", bind_addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
}
