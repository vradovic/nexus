mod app_state;
mod connection_registry;
mod messaging;
mod models;
mod routes;

use std::net::SocketAddr;

use app_state::AppState;
use axum::Router;
use messaging::{ensure_realtime_consumer, ensure_realtime_stream, start_match_found_consumer};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let jwt_secret =
        std::env::var("JWT_SECRET").expect("JWT_SECRET must be set before starting realtime-service");
    let nats_url =
        std::env::var("NATS_URL").expect("NATS_URL must be set before starting realtime-service");
    let nats_client = async_nats::connect(nats_url)
        .await
        .expect("failed to connect to nats");
    ensure_realtime_stream(&nats_client).await;
    ensure_realtime_consumer(&nats_client).await;

    let state = AppState::new(jwt_secret);
    let registry = state.connection_registry.clone();
    tokio::spawn(start_match_found_consumer(nats_client, registry));
    let app: Router = routes::app_router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3004));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    println!("realtime-service listening on ws://{}", addr);

    axum::serve(listener, app).await.expect("server failed");
}
