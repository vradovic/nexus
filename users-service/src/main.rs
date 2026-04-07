mod db;
mod model;
mod routes;

use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let app = routes::app_router();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
