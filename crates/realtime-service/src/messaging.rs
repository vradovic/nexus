use async_nats::{Client, jetstream};
use futures_util::StreamExt;
use nexus_shared::{
    MATCHMAKING_EVENTS_STREAM, MATCH_CONFIRMED_SUBJECT, MATCH_DECLINED_SUBJECT,
    MATCH_FOUND_SUBJECT, MATCH_TIMED_OUT_SUBJECT, MatchConfirmedEvent, MatchDeclinedEvent,
    MatchFoundEvent, MatchTimedOutEvent, ensure_pull_consumer, ensure_stream,
};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::{connection_registry::ConnectionRegistry, models::ServerEvent};

const MATCH_FOUND_CONSUMER: &str = "realtime-service-match-found-consumer";
const MATCH_CONFIRMED_CONSUMER: &str = "realtime-service-match-confirmed-consumer";
const MATCH_DECLINED_CONSUMER: &str = "realtime-service-match-declined-consumer";
const MATCH_TIMED_OUT_CONSUMER: &str = "realtime-service-match-timed-out-consumer";

pub async fn ensure_realtime_stream(nats_client: &Client) {
    ensure_stream(
        nats_client,
        MATCHMAKING_EVENTS_STREAM,
        vec![
            MATCH_FOUND_SUBJECT.to_string(),
            MATCH_CONFIRMED_SUBJECT.to_string(),
            MATCH_DECLINED_SUBJECT.to_string(),
            MATCH_TIMED_OUT_SUBJECT.to_string(),
        ],
    )
    .await
    .expect("failed to create or get matchmaking events stream");
}

pub async fn ensure_realtime_consumers(nats_client: &Client) {
    ensure_pull_consumer(
        nats_client,
        MATCHMAKING_EVENTS_STREAM,
        MATCH_FOUND_CONSUMER,
        MATCH_FOUND_SUBJECT,
    )
    .await
    .expect("failed to create or get realtime match_found consumer");

    ensure_pull_consumer(
        nats_client,
        MATCHMAKING_EVENTS_STREAM,
        MATCH_CONFIRMED_CONSUMER,
        MATCH_CONFIRMED_SUBJECT,
    )
    .await
    .expect("failed to create or get realtime match_confirmed consumer");

    ensure_pull_consumer(
        nats_client,
        MATCHMAKING_EVENTS_STREAM,
        MATCH_DECLINED_CONSUMER,
        MATCH_DECLINED_SUBJECT,
    )
    .await
    .expect("failed to create or get realtime match_declined consumer");

    ensure_pull_consumer(
        nats_client,
        MATCHMAKING_EVENTS_STREAM,
        MATCH_TIMED_OUT_CONSUMER,
        MATCH_TIMED_OUT_SUBJECT,
    )
    .await
    .expect("failed to create or get realtime match_timed_out consumer");
}

pub async fn start_match_found_consumer(
    nats_client: Client,
    connection_registry: ConnectionRegistry,
) {
    start_event_consumer::<MatchFoundEvent, _>(
        nats_client,
        connection_registry,
        MATCH_FOUND_CONSUMER,
        ServerEvent::from,
    )
    .await;
}

pub async fn start_match_confirmed_consumer(
    nats_client: Client,
    connection_registry: ConnectionRegistry,
) {
    start_event_consumer::<MatchConfirmedEvent, _>(
        nats_client,
        connection_registry,
        MATCH_CONFIRMED_CONSUMER,
        ServerEvent::from,
    )
    .await;
}

pub async fn start_match_declined_consumer(
    nats_client: Client,
    connection_registry: ConnectionRegistry,
) {
    start_event_consumer::<MatchDeclinedEvent, _>(
        nats_client,
        connection_registry,
        MATCH_DECLINED_CONSUMER,
        ServerEvent::from,
    )
    .await;
}

pub async fn start_match_timed_out_consumer(
    nats_client: Client,
    connection_registry: ConnectionRegistry,
) {
    start_event_consumer::<MatchTimedOutEvent, _>(
        nats_client,
        connection_registry,
        MATCH_TIMED_OUT_CONSUMER,
        ServerEvent::from,
    )
    .await;
}

async fn start_event_consumer<T, F>(
    nats_client: Client,
    connection_registry: ConnectionRegistry,
    consumer_name: &str,
    to_server_event: F,
) where
    T: DeserializeOwned + Clone + TargetedMatchEvent,
    F: Fn(T) -> ServerEvent + Copy,
{
    let jetstream = jetstream::new(nats_client);
    let stream = jetstream
        .get_stream(MATCHMAKING_EVENTS_STREAM)
        .await
        .expect("failed to get matchmaking events stream");
    let consumer = stream
        .get_consumer::<jetstream::consumer::pull::Config>(consumer_name)
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

        match serde_json::from_slice::<T>(&message.payload) {
            Ok(event) => {
                let recipients = event.player_ids().to_vec();
                let server_event = to_server_event(event);

                for user_id in recipients {
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

trait TargetedMatchEvent {
    fn player_ids(&self) -> &[Uuid];
}

impl TargetedMatchEvent for MatchFoundEvent {
    fn player_ids(&self) -> &[Uuid] {
        &self.player_ids
    }
}

impl TargetedMatchEvent for MatchConfirmedEvent {
    fn player_ids(&self) -> &[Uuid] {
        &self.player_ids
    }
}

impl TargetedMatchEvent for MatchDeclinedEvent {
    fn player_ids(&self) -> &[Uuid] {
        &self.player_ids
    }
}

impl TargetedMatchEvent for MatchTimedOutEvent {
    fn player_ids(&self) -> &[Uuid] {
        &self.player_ids
    }
}
