use gt_simulation::world::GameWorld;
use gt_common::types::WorldConfig;

#[tokio::main]
async fn main() {
    println!("GlobalTelco server starting...");

    let world = GameWorld::new(WorldConfig::default());
    println!("World initialized with config: {:?}", world.config());

    let app = axum::Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    println!("Listening on http://0.0.0.0:3001");
    axum::serve(listener, app).await.unwrap();
}
