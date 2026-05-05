use async_nats::{Client, jetstream};
use futures_util::StreamExt;
use nexus_shared::{
    MATCH_FOUND_SUBJECT, MatchFoundEvent, REALTIME_EVENTS_STREAM, ensure_pull_consumer,
    ensure_stream,
};

use crate::{connection_registry::ConnectionRegistry, models::ServerEvent};

const REALTIME_CONSUMER: &str = "realtime-service-match-found-consumer";

pub async fn ensure_realtime_stream(nats_client: &Client) {
    ensure_stream(
        nats_client,
        REALTIME_EVENTS_STREAM,
        vec![MATCH_FOUND_SUBJECT.to_string()],
    )
        .await
        .expect("failed to create or get realtime events stream");
}

pub async fn ensure_realtime_consumer(nats_client: &Client) {
    ensure_pull_consumer(
        nats_client,
        REALTIME_EVENTS_STREAM,
        REALTIME_CONSUMER,
        MATCH_FOUND_SUBJECT,
    )
        .await
        .expect("failed to create or get realtime consumer");
}

pub async fn start_match_found_consumer(
    nats_client: Client,
    connection_registry: ConnectionRegistry,
) {
    let jetstream = jetstream::new(nats_client);
    let stream = jetstream
        .get_stream(REALTIME_EVENTS_STREAM)
        .await
        .expect("failed to get realtime events stream");
    let consumer = stream
        .get_consumer::<jetstream::consumer::pull::Config>(REALTIME_CONSUMER)
        .await
        .expect("failed to get realtime consumer");
    let mut messages = consumer
        .messages()
        .await
        .expect("failed to open realtime consumer messages");

    while let Some(message_result) = messages.next().await {
        let message = match message_result {
            Ok(message) => message,
            Err(error) => {
                eprintln!("failed to receive realtime event: {}", error);
                continue;
            }
        };

        match serde_json::from_slice::<MatchFoundEvent>(&message.payload) {
            Ok(event) => {
                let server_event = ServerEvent::from(event.clone());

                for user_id in event.user_ids {
                    connection_registry
                        .send_event_to_user(user_id, &server_event)
                        .await;
                }

                if let Err(error) = message.ack().await {
                    eprintln!("failed to ack realtime event: {}", error);
                }
            }
            Err(error) => {
                eprintln!("failed to decode realtime event: {}", error);
            }
        }
    }
}
