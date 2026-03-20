use axum::{
    routing::{get, post},
    Router, Json,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let app = Router::new()
        .route("/health", get(health));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn health() -> &'static str {
    "OK"
}
