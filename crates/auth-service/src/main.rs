mod app_state;
mod db;
mod messaging;
mod models;
mod repositories;
mod routes;
mod services;

use std::net::SocketAddr;

use axum::Router;
use app_state::AppState;
use db::init_db;
use messaging::ensure_registration_stream;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let pool = init_db().await;
    let jwt_secret =
        std::env::var("JWT_SECRET").expect("JWT_SECRET must be set before starting auth-service");
    let nats_url =
        std::env::var("NATS_URL").expect("NATS_URL must be set before starting auth-service");
    let nats_client = async_nats::connect(nats_url)
        .await
        .expect("failed to connect to nats");
    ensure_registration_stream(&nats_client).await;

    let state = AppState::new(pool, jwt_secret, nats_client);
    let app: Router = routes::app_router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    println!("auth-service listening on http://{}", addr);

    axum::serve(listener, app)
        .await
        .expect("server failed");
}
