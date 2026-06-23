use std::sync::Arc;

use axum::extract::ws::Message;
use nexus_shared::nats::{MessageReader, NatsError, NatsMessage};
use serde::Deserialize;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::messaging::{ChannelId, ConnectionId, MessageRouter, MessagingError};

const COMMANDS_CONSUMER: &str = "realtime-service";
const BROADCAST_SUBJECT: &str = "commands.broadcast";
const CHANNEL_BROADCAST_SUBJECT: &str = "commands.broadcast.channels";
const CHANNEL_CREATE_SUBJECT: &str = "commands.channels.create";
const CHANNEL_REMOVE_SUBJECT: &str = "commands.channels.remove";
const CHANNEL_JOIN_SUBJECT: &str = "commands.channels.join";
const CHANNEL_LEAVE_SUBJECT: &str = "commands.channels.leave";

pub fn consumer_name() -> &'static str {
    COMMANDS_CONSUMER
}

#[derive(Debug)]
pub enum RealtimeCommand {
    Broadcast {
        message: Message,
    },
    BroadcastChannels {
        channels: Vec<ChannelId>,
        message: Message,
    },
    CreateChannel {
        channel: ChannelId,
    },
    RemoveChannel {
        channel: ChannelId,
    },
    JoinChannel {
        connection_id: ConnectionId,
        channel: ChannelId,
    },
    LeaveChannel {
        connection_id: ConnectionId,
        channel: ChannelId,
    },
}

#[derive(Clone)]
pub struct RealtimeCommandHandler {
    message_router: Arc<Mutex<MessageRouter>>,
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("nats error: {0}")]
    Nats(#[from] NatsError),
    #[error("messaging error: {0}")]
    Messaging(#[from] MessagingError),
}

#[derive(Debug, Deserialize)]
struct ChannelBroadcastCommand {
    channels: Vec<ChannelId>,
    payload: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct ChannelCommand {
    channel: ChannelId,
}

#[derive(Debug, Deserialize)]
struct ChannelConnectionCommand {
    connection_id: ConnectionId,
    channel: ChannelId,
}

impl RealtimeCommand {
    pub fn from_nats(message: &NatsMessage) -> Result<Option<Self>, CommandError> {
        match message.subject() {
            BROADCAST_SUBJECT => Ok(Some(parse_broadcast(message)?)),
            CHANNEL_BROADCAST_SUBJECT => Ok(Some(parse_channel_broadcast(message)?)),
            CHANNEL_CREATE_SUBJECT => Ok(Some(parse_channel_create(message)?)),
            CHANNEL_REMOVE_SUBJECT => Ok(Some(parse_channel_remove(message)?)),
            CHANNEL_JOIN_SUBJECT => Ok(Some(parse_channel_join(message)?)),
            CHANNEL_LEAVE_SUBJECT => Ok(Some(parse_channel_leave(message)?)),
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
            RealtimeCommand::BroadcastChannels { channels, message } => {
                message_router.broadcast_to_channels(&channels, message);
            }
            RealtimeCommand::CreateChannel { channel } => {
                message_router.create_channel(&channel);
            }
            RealtimeCommand::RemoveChannel { channel } => {
                message_router.remove_channel(&channel)?;
            }
            RealtimeCommand::JoinChannel {
                connection_id,
                channel,
            } => {
                message_router.join_channel(connection_id, &channel)?;
            }
            RealtimeCommand::LeaveChannel {
                connection_id,
                channel,
            } => {
                message_router.leave_channel(connection_id, &channel)?;
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

fn parse_channel_broadcast(message: &NatsMessage) -> Result<RealtimeCommand, CommandError> {
    let command = message.decode::<ChannelBroadcastCommand>()?;

    Ok(RealtimeCommand::BroadcastChannels {
        channels: command.channels,
        message: Message::Binary(command.payload.into()),
    })
}

fn parse_channel_create(message: &NatsMessage) -> Result<RealtimeCommand, CommandError> {
    let command = message.decode::<ChannelCommand>()?;

    Ok(RealtimeCommand::CreateChannel {
        channel: command.channel,
    })
}

fn parse_channel_remove(message: &NatsMessage) -> Result<RealtimeCommand, CommandError> {
    let command = message.decode::<ChannelCommand>()?;

    Ok(RealtimeCommand::RemoveChannel {
        channel: command.channel,
    })
}

fn parse_channel_join(message: &NatsMessage) -> Result<RealtimeCommand, CommandError> {
    let command = message.decode::<ChannelConnectionCommand>()?;

    Ok(RealtimeCommand::JoinChannel {
        connection_id: command.connection_id,
        channel: command.channel,
    })
}

fn parse_channel_leave(message: &NatsMessage) -> Result<RealtimeCommand, CommandError> {
    let command = message.decode::<ChannelConnectionCommand>()?;

    Ok(RealtimeCommand::LeaveChannel {
        connection_id: command.connection_id,
        channel: command.channel,
    })
}
