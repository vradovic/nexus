mod app_state;
mod db;
mod error;
mod models;
mod repositories;
mod routes;
mod services;

use std::net::SocketAddr;

use axum::Router;
use app_state::AppState;
use db::init_db;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let pool = init_db().await;
    let state = AppState::new(pool);
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
