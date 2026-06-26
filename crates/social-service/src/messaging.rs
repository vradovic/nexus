use async_nats::{Client, jetstream};
use futures_util::StreamExt;
use nexus_shared::{
    AppError, ensure_pull_consumer, ensure_stream,
    nats::{COMMANDS_FILTER, COMMANDS_STREAM, EVENTS_FILTER, EVENTS_STREAM},
    publish_json,
};
use serde::Serialize;

use crate::models::{ChatMessage, UserRegisteredEvent};
use crate::repository::UserProfileRepository;
use crate::service;

const USER_REGISTERED_SUBJECT: &str = "events.auth.user_registered";
const USER_REGISTERED_CONSUMER: &str = "social-service-registration-consumer";
const CHANNEL_BROADCAST_SUBJECT: &str = "commands.broadcast.channels";

pub async fn ensure_events_stream(nats_client: &Client) {
    ensure_stream(nats_client, EVENTS_STREAM, vec![EVENTS_FILTER.to_string()])
        .await
        .expect("failed to create or get events stream");
}

pub async fn ensure_commands_stream(nats_client: &Client) {
    ensure_stream(
        nats_client,
        COMMANDS_STREAM,
        vec![COMMANDS_FILTER.to_string()],
    )
    .await
    .expect("failed to create or get commands stream");
}

pub async fn publish_chat_message(
    nats_client: &Client,
    message: &ChatMessage,
) -> Result<(), AppError> {
    let payload = ChatBroadcastPayload {
        r#type: "chat.message",
        channel: &message.channel,
        message,
    };
    let payload =
        serde_json::to_vec(&payload).map_err(|_| AppError::internal("failed to serialize chat"))?;
    let command = ChannelBroadcastCommand {
        channels: vec![message.channel.clone()],
        payload,
    };

    publish_json(nats_client, CHANNEL_BROADCAST_SUBJECT, &command).await
}

pub async fn ensure_registration_consumer(nats_client: &Client) {
    ensure_pull_consumer(
        nats_client,
        EVENTS_STREAM,
        USER_REGISTERED_CONSUMER,
        USER_REGISTERED_SUBJECT,
    )
    .await
    .expect("failed to create or get registration consumer");
}

#[derive(Debug, Serialize)]
struct ChannelBroadcastCommand {
    channels: Vec<String>,
    payload: Vec<u8>,
}

#[derive(Debug, Serialize)]
struct ChatBroadcastPayload<'a> {
    r#type: &'static str,
    channel: &'a str,
    message: &'a ChatMessage,
}

pub async fn start_user_registered_consumer(
    nats_client: Client,
    repository: UserProfileRepository,
) {
    let jetstream = jetstream::new(nats_client);
    let stream = jetstream
        .get_stream(EVENTS_STREAM)
        .await
        .expect("failed to get events stream");
    let consumer = stream
        .get_consumer::<jetstream::consumer::pull::Config>(USER_REGISTERED_CONSUMER)
        .await
        .expect("failed to get registration consumer");
    let mut messages = consumer
        .messages()
        .await
        .expect("failed to open registration consumer messages");

    while let Some(message_result) = messages.next().await {
        let message = match message_result {
            Ok(message) => message,
            Err(error) => {
                eprintln!("failed to receive registration event: {}", error);
                continue;
            }
        };

        match serde_json::from_slice::<UserRegisteredEvent>(&message.payload) {
            Ok(event) => {
                if let Err(error) = service::handle_user_registered(&repository, event).await {
                    eprintln!(
                        "failed to handle user registration event: {}",
                        error.into_response_text()
                    );
                } else if let Err(error) = message.ack().await {
                    eprintln!("failed to ack registration event: {}", error);
                }
            }
            Err(error) => {
                eprintln!("failed to decode registration event: {}", error);
            }
        }
    }
}

trait ErrorText {
    fn into_response_text(self) -> String;
}

impl ErrorText for AppError {
    fn into_response_text(self) -> String {
        let response = axum::response::IntoResponse::into_response(self);
        format!("status={}", response.status())
    }
}
