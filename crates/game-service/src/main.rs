mod nats;

#[tokio::main]
async fn main() {
    load_env();

    let nats_url =
        std::env::var("NATS_URL").expect("NATS_URL must be set before starting game-service");
    let nats_client = async_nats::connect(nats_url)
        .await
        .expect("failed to connect to nats");

    nats::ensure_game_events_consumer(&nats_client).await;

    println!(
        "game-service consuming '{}' from stream '{}'",
        nats::GAME_EVENTS_FILTER,
        nats::EVENTS_STREAM
    );

    let consumer_task = tokio::spawn(nats::start_game_events_consumer(nats_client));
    consumer_task
        .await
        .expect("game events consumer task failed");
}

fn load_env() {
    dotenvy::dotenv().ok();
    dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).ok();
}
