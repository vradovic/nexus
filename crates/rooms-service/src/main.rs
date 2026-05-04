mod app_state;
mod models;
mod redis_store;
mod routes;
mod services;
mod stores;

use std::net::SocketAddr;

use app_state::AppState;
use axum::Router;
use redis::Client;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let redis_url =
        std::env::var("REDIS_URL").expect("REDIS_URL must be set before starting rooms-service");
    let redis_client = Client::open(redis_url).expect("failed to create redis client");

    let state = AppState::new(redis_client);
    let app: Router = routes::app_router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3003));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    println!("rooms-service listening on http://{}", addr);

    axum::serve(listener, app).await.expect("server failed");
}
