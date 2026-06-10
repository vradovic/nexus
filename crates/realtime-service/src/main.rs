mod app;
mod messaging;

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
        )
        .init();

    let address = dotenvy::var("SERVICE_ADDRESS").unwrap();
    let listener = TcpListener::bind(&address).await.unwrap();

    let app = app::build_router();

    tracing::info!(address=%address, "realtime-service started");

    axum::serve(listener, app).await.unwrap();
}
