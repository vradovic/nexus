mod app_state;
mod db;
mod messaging;
mod models;
mod repositories;
mod routes;
mod services;

use std::net::SocketAddr;

use app_state::AppState;
use axum::Router;
use db::init_db;
use messaging::{ensure_registration_consumer, ensure_registration_stream, start_user_registered_consumer};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let pool = init_db().await;
    let nats_url =
        std::env::var("NATS_URL").expect("NATS_URL must be set before starting users-service");
    let nats_client = async_nats::connect(nats_url)
        .await
        .expect("failed to connect to nats");
    ensure_registration_stream(&nats_client).await;
    ensure_registration_consumer(&nats_client).await;

    let state = AppState::new(pool);
    let consumer_repository = state.user_profile_repository.clone();

    tokio::spawn(async move {
        start_user_registered_consumer(nats_client, consumer_repository).await;
    });

    let app: Router = routes::app_router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3002));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    println!("users-service listening on http://{}", addr);

    axum::serve(listener, app)
        .await
        .expect("server failed");
}
