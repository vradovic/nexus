use async_nats::{Client, jetstream};
use futures_util::StreamExt;
use nexus_shared::ensure_pull_consumer;

pub const EVENTS_STREAM: &str = "EVENTS";
pub const GAME_EVENTS_FILTER: &str = "event.game.>";

const GAME_EVENTS_CONSUMER: &str = "game-service-events";

pub async fn ensure_game_events_consumer(nats_client: &Client) {
    ensure_pull_consumer(
        nats_client,
        EVENTS_STREAM,
        GAME_EVENTS_CONSUMER,
        GAME_EVENTS_FILTER,
    )
    .await
    .expect("failed to create or get game events consumer");
}

pub async fn start_game_events_consumer(nats_client: Client) {
    let jetstream = jetstream::new(nats_client);
    let stream = jetstream
        .get_stream(EVENTS_STREAM)
        .await
        .expect("failed to get events stream");
    let consumer = stream
        .get_consumer::<jetstream::consumer::pull::Config>(GAME_EVENTS_CONSUMER)
        .await
        .expect("failed to get game events consumer");
    let mut messages = consumer
        .messages()
        .await
        .expect("failed to open game events consumer messages");

    while let Some(message_result) = messages.next().await {
        let message = match message_result {
            Ok(message) => message,
            Err(error) => {
                eprintln!("failed to receive game event: {}", error);
                continue;
            }
        };

        println!(
            "game-service received {}: {}",
            message.subject,
            String::from_utf8_lossy(&message.payload)
        );

        if let Err(error) = message.ack().await {
            eprintln!("failed to ack game event: {}", error);
        }
    }
}
