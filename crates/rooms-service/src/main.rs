mod app_state;
mod messaging;
mod models;
mod redis_store;
mod routes;
mod services;
mod stores;

use std::net::SocketAddr;

use app_state::AppState;
use axum::Router;
use messaging::ensure_realtime_stream;
use redis::Client;
use services::matchmaking_service::run_matchmaking_loop;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let redis_url =
        std::env::var("REDIS_URL").expect("REDIS_URL must be set before starting rooms-service");
    let redis_client = Client::open(redis_url).expect("failed to create redis client");
    let jwt_secret =
        std::env::var("JWT_SECRET").expect("JWT_SECRET must be set before starting rooms-service");
    let nats_url =
        std::env::var("NATS_URL").expect("NATS_URL must be set before starting rooms-service");
    let nats_client = async_nats::connect(nats_url)
        .await
        .expect("failed to connect to nats");
    ensure_realtime_stream(&nats_client).await;

    let state = AppState::new(redis_client, jwt_secret, nats_client);
    tokio::spawn(run_matchmaking_loop(state.clone()));
    let app: Router = routes::app_router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3003));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    println!("rooms-service listening on http://{}", addr);

    axum::serve(listener, app).await.expect("server failed");
}
