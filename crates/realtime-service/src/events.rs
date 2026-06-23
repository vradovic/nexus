use std::sync::Arc;

use nexus_shared::{
    MATCH_CHANNEL_READY_SUBJECT, MATCH_CONFIRMED_SUBJECT, MatchConfirmedEvent,
    nats::{MessageReader, NatsError, NatsMessage},
};

use crate::{app::AppState, messaging::MessagingError};

const EVENTS_CONSUMER: &str = "realtime-service-matchmaking";

pub fn consumer_name() -> &'static str {
    EVENTS_CONSUMER
}

pub async fn run_event_loop(mut reader: MessageReader, state: Arc<AppState>) {
    tracing::info!("started realtime event loop");

    loop {
        let message = match reader.next().await {
            Ok(Some(message)) => message,
            Ok(None) => {
                tracing::warn!("realtime event reader ended");
                return;
            }
            Err(error) => {
                tracing::error!(%error, "failed to read realtime event");
                continue;
            }
        };

        let subject = message.subject().to_string();

        if let Err(error) = handle_event(&state, &message).await {
            tracing::error!(%error, subject = %subject, "failed to handle realtime event");
        }

        if let Err(error) = message.ack().await {
            tracing::error!(%error, subject = %subject, "failed to ack realtime event");
        }
    }
}

async fn handle_event(state: &AppState, message: &NatsMessage) -> Result<(), EventError> {
    match message.subject() {
        MATCH_CONFIRMED_SUBJECT => handle_match_confirmed(state, message.decode()?).await,
        _ => Ok(()),
    }
}

async fn handle_match_confirmed(
    state: &AppState,
    event: MatchConfirmedEvent,
) -> Result<(), EventError> {
    if let Some(ready_event) = state.create_match_channel(event).await? {
        state
            .publish_json(MATCH_CHANNEL_READY_SUBJECT, &ready_event)
            .await?;
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum EventError {
    #[error("nats error: {0}")]
    Nats(#[from] NatsError),
    #[error("messaging error: {0}")]
    Messaging(#[from] MessagingError),
}
