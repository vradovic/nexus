use std::collections::{HashMap, HashSet};

use axum::extract::ws::Message;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

pub const DEFAULT_ROOM_ID: &str = "default";

pub type Sender = UnboundedSender<Message>;
pub type RoomId = String;
pub type ConnectionId = Uuid;

#[derive(Debug)]
pub struct MessageRouter {
    connections: HashMap<ConnectionId, Sender>,
    rooms: HashMap<RoomId, HashSet<ConnectionId>>,
    connection_rooms: HashMap<ConnectionId, HashSet<RoomId>>,
}

#[derive(Debug, Error)]
pub enum MessagingError {
    #[error("connection {conn_id} not found")]
    ConnectionNotFound { conn_id: ConnectionId },
}

impl MessageRouter {
    pub fn add_connection(
        &mut self,
        conn_id: ConnectionId,
        tx: Sender,
        room_id: &str,
    ) -> Result<(), MessagingError> {
        self.connections.insert(conn_id, tx);
        if let Err(error) = self.join_room(conn_id, room_id) {
            self.connections.remove(&conn_id);
            return Err(error);
        }

        Ok(())
    }

    pub fn remove_connection(&mut self, conn_id: ConnectionId) {
        self.connections.remove(&conn_id);

        let Some(room_ids) = self.connection_rooms.remove(&conn_id) else {
            return;
        };

        for room_id in room_ids {
            if let Some(room) = self.rooms.get_mut(&room_id) {
                room.remove(&conn_id);
            }
        }
    }

    pub fn broadcast(&mut self, message: Message) {
        let conn_ids = self.connections.keys().copied().collect::<Vec<_>>();
        self.send_to_connections(conn_ids, message);
    }

    fn send_to_connections(&mut self, conn_ids: Vec<ConnectionId>, message: Message) {
        let mut stale_conns = Vec::<ConnectionId>::new();

        for conn_id in conn_ids {
            let tx = match self.connections.get(&conn_id) {
                Some(tx) => tx,
                None => {
                    stale_conns.push(conn_id);
                    continue;
                }
            };

            if let Err(error) = tx.send(message.clone()) {
                tracing::error!(%error, "failed to write to channel");
                stale_conns.push(conn_id);
            }
        }

        for conn_id in stale_conns {
            self.remove_connection(conn_id);
        }
    }

    pub fn join_room(
        &mut self,
        conn_id: ConnectionId,
        room_id: &str,
    ) -> Result<(), MessagingError> {
        if !self.connections.contains_key(&conn_id) {
            return Err(MessagingError::ConnectionNotFound { conn_id });
        }

        let room = self.rooms.entry(room_id.to_string()).or_default();
        room.insert(conn_id);

        self.connection_rooms
            .entry(conn_id)
            .or_default()
            .insert(room_id.to_string());

        Ok(())
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        let mut rooms = HashMap::<RoomId, HashSet<ConnectionId>>::new();
        rooms.insert(DEFAULT_ROOM_ID.to_string(), HashSet::<ConnectionId>::new());

        Self {
            rooms,
            connections: Default::default(),
            connection_rooms: Default::default(),
        }
    }
}
