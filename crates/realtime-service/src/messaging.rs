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
    #[error("room {room_id} not found")]
    RoomNotFound { room_id: RoomId },
    #[error("default room cannot be removed")]
    CannotRemoveDefaultRoom,
}

impl MessageRouter {
    pub fn add_connection(
        &mut self,
        conn_id: ConnectionId,
        tx: Sender,
    ) -> Result<(), MessagingError> {
        self.connections.insert(conn_id, tx);
        if let Err(error) = self.join_room(conn_id, DEFAULT_ROOM_ID) {
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

    pub fn broadcast_to_rooms(&mut self, room_ids: &[RoomId], message: Message) {
        let mut conn_ids = HashSet::<ConnectionId>::new();

        for room_id in room_ids {
            if let Some(room) = self.rooms.get(room_id) {
                conn_ids.extend(room.iter().copied());
            }
        }

        self.send_to_connections(conn_ids.into_iter().collect(), message);
    }

    pub fn create_room(&mut self, room_id: &str) {
        self.rooms.entry(room_id.to_string()).or_default();
    }

    pub fn remove_room(&mut self, room_id: &str) -> Result<(), MessagingError> {
        if room_id == DEFAULT_ROOM_ID {
            return Err(MessagingError::CannotRemoveDefaultRoom);
        }

        let Some(conn_ids) = self.rooms.remove(room_id) else {
            return Ok(());
        };

        for conn_id in conn_ids {
            if let Some(room_ids) = self.connection_rooms.get_mut(&conn_id) {
                room_ids.remove(room_id);
            }
        }

        Ok(())
    }

    pub fn all_rooms(&self) -> Vec<RoomId> {
        let mut room_ids = self.rooms.keys().cloned().collect::<Vec<_>>();
        room_ids.sort();
        room_ids
    }

    pub fn rooms_for_connection(
        &self,
        conn_id: ConnectionId,
    ) -> Result<Vec<RoomId>, MessagingError> {
        let Some(room_ids) = self.connection_rooms.get(&conn_id) else {
            return Err(MessagingError::ConnectionNotFound { conn_id });
        };

        let mut room_ids = room_ids.iter().cloned().collect::<Vec<_>>();
        room_ids.sort();
        Ok(room_ids)
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

        let Some(room) = self.rooms.get_mut(room_id) else {
            return Err(MessagingError::RoomNotFound {
                room_id: room_id.to_string(),
            });
        };

        room.insert(conn_id);

        self.connection_rooms
            .entry(conn_id)
            .or_default()
            .insert(room_id.to_string());

        Ok(())
    }

    pub fn leave_room(
        &mut self,
        conn_id: ConnectionId,
        room_id: &str,
    ) -> Result<(), MessagingError> {
        if !self.connections.contains_key(&conn_id) {
            return Err(MessagingError::ConnectionNotFound { conn_id });
        }

        let Some(room) = self.rooms.get_mut(room_id) else {
            return Err(MessagingError::RoomNotFound {
                room_id: room_id.to_string(),
            });
        };

        room.remove(&conn_id);

        if let Some(room_ids) = self.connection_rooms.get_mut(&conn_id) {
            room_ids.remove(room_id);
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::sync::mpsc;

    #[test]
    fn broadcast_to_rooms_only_sends_to_room_members() {
        let mut router = MessageRouter::default();
        let alpha_conn = Uuid::new_v4();
        let beta_conn = Uuid::new_v4();
        let (alpha_tx, mut alpha_rx) = mpsc::unbounded_channel();
        let (beta_tx, mut beta_rx) = mpsc::unbounded_channel();

        router
            .add_connection(alpha_conn, alpha_tx)
            .expect("alpha connection is added");
        router
            .add_connection(beta_conn, beta_tx)
            .expect("beta connection is added");
        router.create_room("alpha");
        router
            .join_room(alpha_conn, "alpha")
            .expect("alpha connection joins alpha room");

        router.broadcast_to_rooms(&["alpha".to_string()], Message::Text("hello".into()));

        match alpha_rx.try_recv().expect("alpha receives message") {
            Message::Text(text) => assert_eq!(text.as_str(), "hello"),
            message => panic!("unexpected message: {message:?}"),
        }
        assert!(beta_rx.try_recv().is_err());
    }

    #[test]
    fn room_lifecycle_updates_room_membership_indexes() {
        let mut router = MessageRouter::default();
        let conn_id = Uuid::new_v4();
        let (tx, _rx) = mpsc::unbounded_channel();

        router
            .add_connection(conn_id, tx)
            .expect("connection is added");
        router.create_room("match-1");
        router
            .join_room(conn_id, "match-1")
            .expect("connection joins room");

        assert_eq!(
            router.rooms_for_connection(conn_id).expect("rooms exist"),
            vec!["default".to_string(), "match-1".to_string()]
        );
        assert_eq!(
            router.all_rooms(),
            vec!["default".to_string(), "match-1".to_string()]
        );

        router
            .leave_room(conn_id, "match-1")
            .expect("connection leaves room");
        assert_eq!(
            router.rooms_for_connection(conn_id).expect("rooms exist"),
            vec!["default".to_string()]
        );

        router
            .join_room(conn_id, "match-1")
            .expect("connection rejoins room");
        router.remove_room("match-1").expect("room is removed");
        assert_eq!(
            router.rooms_for_connection(conn_id).expect("rooms exist"),
            vec!["default".to_string()]
        );
        assert_eq!(router.all_rooms(), vec!["default".to_string()]);
    }
}
