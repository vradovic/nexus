use std::sync::Arc;

use axum::extract::ws::Message;
use nexus_shared::nats::{MessageReader, NatsError, NatsMessage};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::messaging::MessageRouter;

const COMMANDS_CONSUMER: &str = "realtime-service";
const BROADCAST_SUBJECT: &str = "commands.broadcast";

pub fn consumer_name() -> &'static str {
    COMMANDS_CONSUMER
}

#[derive(Debug)]
pub enum RealtimeCommand {
    Broadcast { message: Message },
}

#[derive(Clone)]
pub struct RealtimeCommandHandler {
    message_router: Arc<Mutex<MessageRouter>>,
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("nats error: {0}")]
    Nats(#[from] NatsError),
}

impl RealtimeCommand {
    pub fn from_nats(message: &NatsMessage) -> Result<Option<Self>, CommandError> {
        match message.subject() {
            BROADCAST_SUBJECT => Ok(Some(parse_broadcast(message)?)),
            _ => Ok(None),
        }
    }
}

impl RealtimeCommandHandler {
    pub fn new(message_router: Arc<Mutex<MessageRouter>>) -> Self {
        Self { message_router }
    }

    pub async fn handle(&self, command: RealtimeCommand) -> Result<(), CommandError> {
        let mut message_router = self.message_router.lock().await;

        match command {
            RealtimeCommand::Broadcast { message } => {
                message_router.broadcast(message);
            }
        }

        Ok(())
    }
}

pub async fn run_command_loop(mut reader: MessageReader, handler: RealtimeCommandHandler) {
    tracing::info!("started realtime command loop");

    loop {
        let message = match reader.next().await {
            Ok(Some(message)) => message,
            Ok(None) => {
                tracing::warn!("realtime command reader ended");
                return;
            }
            Err(error) => {
                tracing::error!(%error, "failed to read realtime command");
                continue;
            }
        };

        let subject = message.subject().to_string();

        match RealtimeCommand::from_nats(&message) {
            Ok(Some(command)) => {
                if let Err(error) = handler.handle(command).await {
                    tracing::error!(%error, subject = %subject, "failed to handle realtime command");
                }
            }
            Ok(None) => {
                tracing::debug!(subject = %subject, "ignoring unsupported realtime command");
            }
            Err(error) => {
                tracing::error!(%error, subject = %subject, "failed to parse realtime command");
            }
        }

        if let Err(error) = message.ack().await {
            tracing::error!(%error, subject = %subject, "failed to ack realtime command");
        }
    }
}

fn parse_broadcast(message: &NatsMessage) -> Result<RealtimeCommand, CommandError> {
    Ok(RealtimeCommand::Broadcast {
        message: Message::Binary(message.payload_bytes().clone()),
    })
}
