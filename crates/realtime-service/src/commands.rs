use std::sync::Arc;

use axum::extract::ws::Message;
use nexus_shared::nats::{MessageReader, NatsError, NatsMessage};
use serde::Deserialize;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::messaging::{ConnectionId, MessageRouter, MessagingError, RoomId};

const COMMANDS_CONSUMER: &str = "realtime-service";
const BROADCAST_SUBJECT: &str = "commands.broadcast";
const ROOM_BROADCAST_SUBJECT: &str = "commands.broadcast.rooms";
const ROOM_CREATE_SUBJECT: &str = "commands.rooms.create";
const ROOM_REMOVE_SUBJECT: &str = "commands.rooms.remove";
const ROOM_JOIN_SUBJECT: &str = "commands.rooms.join";
const ROOM_LEAVE_SUBJECT: &str = "commands.rooms.leave";

pub fn consumer_name() -> &'static str {
    COMMANDS_CONSUMER
}

#[derive(Debug)]
pub enum RealtimeCommand {
    Broadcast {
        message: Message,
    },
    BroadcastRooms {
        rooms: Vec<RoomId>,
        message: Message,
    },
    CreateRoom {
        room: RoomId,
    },
    RemoveRoom {
        room: RoomId,
    },
    JoinRoom {
        connection_id: ConnectionId,
        room: RoomId,
    },
    LeaveRoom {
        connection_id: ConnectionId,
        room: RoomId,
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
struct RoomBroadcastCommand {
    rooms: Vec<RoomId>,
    payload: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct RoomCommand {
    room: RoomId,
}

#[derive(Debug, Deserialize)]
struct RoomConnectionCommand {
    connection_id: ConnectionId,
    room: RoomId,
}

impl RealtimeCommand {
    pub fn from_nats(message: &NatsMessage) -> Result<Option<Self>, CommandError> {
        match message.subject() {
            BROADCAST_SUBJECT => Ok(Some(parse_broadcast(message)?)),
            ROOM_BROADCAST_SUBJECT => Ok(Some(parse_room_broadcast(message)?)),
            ROOM_CREATE_SUBJECT => Ok(Some(parse_room_create(message)?)),
            ROOM_REMOVE_SUBJECT => Ok(Some(parse_room_remove(message)?)),
            ROOM_JOIN_SUBJECT => Ok(Some(parse_room_join(message)?)),
            ROOM_LEAVE_SUBJECT => Ok(Some(parse_room_leave(message)?)),
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
            RealtimeCommand::BroadcastRooms { rooms, message } => {
                message_router.broadcast_to_rooms(&rooms, message);
            }
            RealtimeCommand::CreateRoom { room } => {
                message_router.create_room(&room);
            }
            RealtimeCommand::RemoveRoom { room } => {
                message_router.remove_room(&room)?;
            }
            RealtimeCommand::JoinRoom {
                connection_id,
                room,
            } => {
                message_router.join_room(connection_id, &room)?;
            }
            RealtimeCommand::LeaveRoom {
                connection_id,
                room,
            } => {
                message_router.leave_room(connection_id, &room)?;
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

fn parse_room_broadcast(message: &NatsMessage) -> Result<RealtimeCommand, CommandError> {
    let command = message.decode::<RoomBroadcastCommand>()?;

    Ok(RealtimeCommand::BroadcastRooms {
        rooms: command.rooms,
        message: Message::Binary(command.payload.into()),
    })
}

fn parse_room_create(message: &NatsMessage) -> Result<RealtimeCommand, CommandError> {
    let command = message.decode::<RoomCommand>()?;

    Ok(RealtimeCommand::CreateRoom { room: command.room })
}

fn parse_room_remove(message: &NatsMessage) -> Result<RealtimeCommand, CommandError> {
    let command = message.decode::<RoomCommand>()?;

    Ok(RealtimeCommand::RemoveRoom { room: command.room })
}

fn parse_room_join(message: &NatsMessage) -> Result<RealtimeCommand, CommandError> {
    let command = message.decode::<RoomConnectionCommand>()?;

    Ok(RealtimeCommand::JoinRoom {
        connection_id: command.connection_id,
        room: command.room,
    })
}

fn parse_room_leave(message: &NatsMessage) -> Result<RealtimeCommand, CommandError> {
    let command = message.decode::<RoomConnectionCommand>()?;

    Ok(RealtimeCommand::LeaveRoom {
        connection_id: command.connection_id,
        room: command.room,
    })
}
