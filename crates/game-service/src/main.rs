use crate::rhai::ScriptEngine;

mod mappings;
mod nats;
mod rhai;

#[tokio::main]
async fn main() {
    load_env();
    init_tracing();

    let nats_url =
        std::env::var("NATS_URL").expect("NATS_URL must be set before starting game-service");
    tracing::info!(nats_url = %nats_url, "connecting to nats");

    let nats_client = async_nats::connect(nats_url)
        .await
        .expect("failed to connect to nats");
    tracing::info!("connected to nats");

    let script_path =
        std::env::var("GAME_SCRIPT_PATH").unwrap_or_else(|_| rhai::DEFAULT_SCRIPT_PATH.to_string());
    tracing::debug!(script_path, "initializing script engine");

    let engine = ScriptEngine::new(&script_path);

    tracing::info!("starting game event consumer");
    nats::start_consumer(nats_client, move |event| engine.handle_event(event)).await;
}

fn load_env() {
    dotenvy::dotenv().ok();
    dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).ok();
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "game_service=info,async_nats=warn".to_string()),
        )
        .init();
}
