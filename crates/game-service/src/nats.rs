use async_nats::{Client, jetstream};
use futures_util::StreamExt;

use crate::{rhai::Event, rhai::HookResult};

const EVENTS_STREAM: &str = "EVENTS";
const EVENTS_FILTER: &str = "events.>";
const EVENTS_CONSUMER: &str = "game-service";

pub async fn start_consumer<F>(nats_client: Client, event_handler: F)
where
    F: Fn(Event) -> HookResult,
{
    tracing::info!(
        stream = EVENTS_STREAM,
        consumer = EVENTS_CONSUMER,
        filter = EVENTS_FILTER,
        "starting nats consumer"
    );

    let jetstream = jetstream::new(nats_client);

    let stream = jetstream
        .get_stream(EVENTS_STREAM)
        .await
        .expect("failed to get events stream");

    let consumer: jetstream::consumer::Consumer<jetstream::consumer::pull::Config> = stream
        .get_or_create_consumer::<jetstream::consumer::pull::Config>(
            EVENTS_CONSUMER,
            jetstream::consumer::pull::Config {
                durable_name: Some(EVENTS_CONSUMER.to_string()),
                filter_subject: EVENTS_FILTER.to_string(),
                ..Default::default()
            },
        )
        .await
        .expect("failed to get game events consumer");

    tracing::info!(
        stream = EVENTS_STREAM,
        consumer = EVENTS_CONSUMER,
        filter = EVENTS_FILTER,
        "nats consumer ready"
    );

    let mut messages = consumer
        .messages()
        .await
        .expect("failed to open game events consumer messages");

    while let Some(message_result) = messages.next().await {
        let message = match message_result {
            Ok(message) => message,
            Err(error) => {
                tracing::error!(error = %error, "failed to receive game event");
                continue;
            }
        };

        let event = Event {
            subject: message.subject.to_string(),
            payload: message.payload.clone(),
        };

        tracing::debug!(
            subject = %event.subject,
            payload_size = event.payload.len(),
            "received game event"
        );

        if let Err(error) = event_handler(event) {
            tracing::error!(error = %error, "failed to handle game event");
            continue;
        }

        if let Err(error) = message.ack().await {
            tracing::error!(error = %error, "failed to ack game event");
        } else {
            tracing::debug!("acked game event");
        }
    }
}
