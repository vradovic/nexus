use crate::game::Game;

mod commands;
mod engine;
mod game;
mod mappings;

#[tokio::main]
async fn main() {
    load_env();
    init_tracing();

    let nats_url =
        std::env::var("NATS_URL").expect("NATS_URL must be set before starting game-service");
    tracing::info!(nats_url = %nats_url, "connecting to nats");

    let nats = nexus_shared::nats::NatsAdapter::new(&nats_url)
        .await
        .expect("failed to initialize nats adapter");
    tracing::info!("connected to nats");

    let script_path = std::env::var("GAME_SCRIPT_PATH")
        .unwrap_or_else(|_| engine::DEFAULT_SCRIPT_PATH.to_string());
    tracing::debug!(script_path, "initializing script engine");

    let mut game = Game::new(nats, &script_path)
        .await
        .expect("failed to initialize game");

    game.run().await.expect("game loop failed");
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
