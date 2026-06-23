mod app;
mod commands;
mod events;
mod messaging;

use std::sync::Arc;

use commands::RealtimeCommandHandler;
use nexus_shared::nats::NatsAdapter;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    load_env();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
        )
        .init();

    let nats_url =
        dotenvy::var("NATS_URL").expect("NATS_URL must be set before starting realtime-service");
    let jwt_secret = dotenvy::var("JWT_SECRET")
        .expect("JWT_SECRET must be set before starting realtime-service");
    tracing::info!(nats_url = %nats_url, "connecting to nats");

    let nats = NatsAdapter::new(&nats_url)
        .await
        .expect("failed to initialize nats adapter");
    let command_reader = nats
        .commands_reader(commands::consumer_name())
        .await
        .expect("failed to initialize realtime command reader");
    let event_reader = nats
        .events_reader(events::consumer_name())
        .await
        .expect("failed to initialize realtime event reader");
    tracing::info!("connected to nats");

    let state = Arc::new(app::AppState::new(nats, jwt_secret));
    let command_handler = RealtimeCommandHandler::new(state.message_router());
    tokio::spawn(commands::run_command_loop(command_reader, command_handler));
    tokio::spawn(events::run_event_loop(event_reader, Arc::clone(&state)));

    let address = dotenvy::var("SERVICE_ADDRESS").unwrap();
    let listener = TcpListener::bind(&address).await.unwrap();

    let app = app::build_router(state);

    tracing::info!(address=%address, "realtime-service started");

    axum::serve(listener, app).await.unwrap();
}

fn load_env() {
    dotenvy::dotenv().ok();
    dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).ok();
}
