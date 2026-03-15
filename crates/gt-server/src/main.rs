mod auth;
mod config;
mod db;
mod oauth;
mod r2;
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
#[cfg(feature = "postgres")]
use tracing::warn;

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

    // Initialize R2 storage if configured
    #[cfg(feature = "r2")]
    let r2_storage = if let (Some(acct), Some(key), Some(secret), Some(bucket)) = (
        config.r2_account_id.as_ref(),
        config.r2_access_key_id.as_ref(),
        config.r2_secret_access_key.as_ref(),
        config.r2_bucket_name.as_ref(),
    ) {
        match r2::R2Storage::new(acct, key, secret, bucket) {
            Ok(storage) => {
                info!("R2 storage: connected (bucket: {})", bucket);
                Some(storage)
            }
            Err(e) => {
                warn!("R2 storage: failed to initialize ({e}), falling back to PostgreSQL");
                None
            }
        }
    } else {
        info!("R2 storage: disabled (fallback to PostgreSQL)");
        None
    };

    // Create shared state
    let mut app_state = AppState::new(config.auth.clone(), database)
        .with_oauth(config.oauth.clone())
        .with_cf_reset_url(config.cf_reset_worker_url.clone());

    #[cfg(feature = "r2")]
    {
        app_state = app_state.with_r2(r2_storage);
    }

    let state = Arc::new(app_state);

    // Restore persisted worlds from database, or create a default world
    #[cfg(feature = "postgres")]
    {
        let mut restored_count = 0usize;
        if let Some(db) = state.db.as_ref() {
            match db.list_active_worlds().await {
                Ok(worlds) if !worlds.is_empty() => {
                    for world_row in &worlds {
                        // Attempt to load the latest snapshot for this world
                        // Try R2 first (metadata from DB points to R2 key), fall back to DB blob
                        let game_world = 'restore: {
                            // Try R2-backed restore
                            #[cfg(feature = "r2")]
                            if let Some(ref r2) = state.r2 {
                                match db.load_latest_snapshot_metadata(world_row.id).await {
                                    Ok(Some(meta)) => {
                                        match r2.get(&meta.r2_key).await {
                                            Ok(Some(data)) => {
                                                match gt_simulation::world::GameWorld::load_game_binary(&data) {
                                                    Ok(w) => {
                                                        info!(
                                                            "Restored world '{}' from R2 snapshot at tick {}",
                                                            world_row.name, meta.tick
                                                        );
                                                        break 'restore Some(w);
                                                    }
                                                    Err(e) => {
                                                        warn!(
                                                            "Failed to deserialize R2 snapshot for world '{}': {e}",
                                                            world_row.name
                                                        );
                                                    }
                                                }
                                            }
                                            Ok(None) => {
                                                warn!(
                                                    "R2 key '{}' not found for world '{}', trying DB blob",
                                                    meta.r2_key, world_row.name
                                                );
                                            }
                                            Err(e) => {
                                                warn!(
                                                    "R2 GET failed for world '{}': {e}, trying DB blob",
                                                    world_row.name
                                                );
                                            }
                                        }
                                    }
                                    Ok(None) => {}
                                    Err(e) => {
                                        warn!("Failed to load R2 snapshot metadata for world '{}': {e}", world_row.name);
                                    }
                                }
                            }

                            // Fall back to DB blob
                            match db.load_latest_snapshot(world_row.id).await {
                                Ok(Some(snapshot)) => {
                                    match gt_simulation::world::GameWorld::load_game_binary(&snapshot.state_data) {
                                        Ok(w) => {
                                            info!(
                                                "Restored world '{}' from DB snapshot at tick {}",
                                                world_row.name, snapshot.tick
                                            );
                                            Some(w)
                                        }
                                        Err(e) => {
                                            warn!(
                                                "Failed to deserialize snapshot for world '{}': {e}",
                                                world_row.name
                                            );
                                            None
                                        }
                                    }
                                }
                                Ok(None) => {
                                    info!("No snapshot for world '{}', creating fresh", world_row.name);
                                    None
                                }
                                Err(e) => {
                                    warn!(
                                        "Failed to load snapshot for world '{}': {e}",
                                        world_row.name
                                    );
                                    None
                                }
                            }
                        };

                        // Parse the config from DB or use default
                        let config: gt_common::types::WorldConfig =
                            serde_json::from_value(world_row.config_json.clone())
                                .unwrap_or_default();

                        // Create the WorldInstance (either from snapshot or fresh)
                        let instance = std::sync::Arc::new(state::WorldInstance::new(
                            world_row.id,
                            world_row.name.clone(),
                            config,
                            world_row.max_players as u32,
                        ));

                        // If we have a restored world, swap it in
                        if let Some(restored) = game_world {
                            *instance.world.lock().await = restored;
                        }

                        state.worlds.write().await.insert(world_row.id, instance);
                        restored_count += 1;
                    }
                    info!("Restored {} world(s) from database", restored_count);
                }
                Ok(_) => {
                    info!("No active worlds in database");
                }
                Err(e) => {
                    warn!("Failed to list active worlds from database: {e}");
                }
            }
        }

        // If no worlds were restored, create a default one
        if restored_count == 0 {
            let default_world_id = state
                .create_world(
                    config.default_world_name.clone(),
                    gt_common::types::WorldConfig::default(),
                    config.default_max_players,
                )
                .await;
            info!("Created default world: {}", default_world_id);
        }
    }

    #[cfg(not(feature = "postgres"))]
    {
        let default_world_id = state
            .create_world(
                config.default_world_name.clone(),
                gt_common::types::WorldConfig::default(),
                config.default_max_players,
            )
            .await;
        info!("Created default world: {}", default_world_id);
    }

    // Start tick loops for all worlds
    tick::start_all_tick_loops(&state).await;

    // CORS — restrict in production when CORS_ORIGIN or CORS_ORIGINS is set
    let cors_origins: Vec<HeaderValue> = {
        let mut origins = Vec::new();
        // Support CORS_ORIGINS (comma-separated) first, then CORS_ORIGIN (single) for backward compat
        if let Ok(multi) = std::env::var("CORS_ORIGINS") {
            for origin in multi.split(',') {
                let trimmed = origin.trim();
                if !trimmed.is_empty() {
                    if let Ok(val) = trimmed.parse::<HeaderValue>() {
                        origins.push(val);
                    }
                }
            }
        }
        if let Ok(single) = std::env::var("CORS_ORIGIN") {
            let trimmed = single.trim();
            if !trimmed.is_empty() {
                if let Ok(val) = trimmed.parse::<HeaderValue>() {
                    if !origins.iter().any(|o| o == &val) {
                        origins.push(val);
                    }
                }
            }
        }
        origins
    };

    let allowed_headers = [
        header::AUTHORIZATION,
        header::CONTENT_TYPE,
        header::UPGRADE,
        header::CONNECTION,
        HeaderName::from_static("sec-websocket-key"),
        HeaderName::from_static("sec-websocket-version"),
        HeaderName::from_static("sec-websocket-protocol"),
        HeaderName::from_static("x-admin-key"),
    ];

    let cors = if cors_origins.is_empty() {
        info!("CORS open (no CORS_ORIGIN/CORS_ORIGINS set -- dev mode)");
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        info!("CORS restricted to {} origin(s)", cors_origins.len());
        for o in &cors_origins {
            info!("  CORS origin: {}", o.to_str().unwrap_or("?"));
        }
        CorsLayer::new()
            .allow_origin(cors_origins)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
            .allow_headers(allowed_headers)
    };

    // Build router
    let app = routes::create_router(Arc::clone(&state), config.tile_dir.clone())
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let bind_addr = config.bind_addr();
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind to {}: {}", bind_addr, e));
    info!("Listening on http://{}", bind_addr);
    info!("WebSocket endpoint: ws://{}/ws", bind_addr);
    info!("REST API: http://{}/api", bind_addr);
    info!("Health check: http://{}/health", bind_addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .expect("Server terminated unexpectedly");
}
