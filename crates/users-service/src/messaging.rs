use async_nats::{Client, jetstream};
use futures_util::StreamExt;
use nexus_shared::{AppError, ensure_pull_consumer, ensure_stream};

use crate::models::UserRegisteredEvent;
use crate::repositories::user_profile_repository::UserProfileRepository;
use crate::services::user_service;

const USER_REGISTERED_STREAM: &str = "AUTH_EVENTS";
const USER_REGISTERED_SUBJECT: &str = "auth.user.registered";
const USER_REGISTERED_CONSUMER: &str = "users-service-registration-consumer";

pub async fn ensure_registration_stream(nats_client: &Client) {
    ensure_stream(
        nats_client,
        USER_REGISTERED_STREAM,
        vec![USER_REGISTERED_SUBJECT.to_string()],
    )
        .await
        .expect("failed to create or get auth events stream");
}

pub async fn ensure_registration_consumer(nats_client: &Client) {
    ensure_pull_consumer(
        nats_client,
        USER_REGISTERED_STREAM,
        USER_REGISTERED_CONSUMER,
        USER_REGISTERED_SUBJECT,
    )
        .await
        .expect("failed to create or get registration consumer");
}

pub async fn start_user_registered_consumer(nats_client: Client, repository: UserProfileRepository) {
    let jetstream = jetstream::new(nats_client);
    let stream = jetstream
        .get_stream(USER_REGISTERED_STREAM)
        .await
        .expect("failed to get auth events stream");
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
                if let Err(error) = user_service::handle_user_registered(&repository, event).await {
                    eprintln!("failed to handle user registration event: {}", error.into_response_text());
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
