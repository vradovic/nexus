use crate::rhai::ScriptEngine;

mod nats;
mod rhai;

#[tokio::main]
async fn main() {
    load_env();

    let nats_url =
        std::env::var("NATS_URL").expect("NATS_URL must be set before starting game-service");
    let nats_client = async_nats::connect(nats_url)
        .await
        .expect("failed to connect to nats");

    let script_path =
        std::env::var("GAME_SCRIPT_PATH").unwrap_or_else(|_| rhai::DEFAULT_SCRIPT_PATH.to_string());

    let engine = ScriptEngine::new(&script_path);

    nats::start_consumer(nats_client, move |event| engine.handle_event(event)).await;
}

fn load_env() {
    dotenvy::dotenv().ok();
    dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).ok();
}
