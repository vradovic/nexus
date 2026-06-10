use std::collections::HashMap;

use axum::{
    body::Bytes,
    extract::ws::{Message, WebSocket},
};
use futures_util::{SinkExt, stream::SplitSink};
use thiserror::Error;

type Conn = SplitSink<WebSocket, Message>;
type Room = Vec<Conn>;

#[derive(Default, Debug)]
pub struct RoomRegistry {
    rooms: HashMap<String, Room>,
}

#[derive(Debug, Error)]
pub enum RoomError {
    #[error("room {room_id} not found")]
    RoomNotFound { room_id: String },
}

impl RoomRegistry {
    // pub fn add_to_room(&mut self, conn: Conn, room_id: &str) {
    //     self.rooms.insert(room_id.to_string(), conn);
    // }

    pub async fn write_to_room(&mut self, room_id: &str, payload: Bytes) -> Result<(), RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::RoomNotFound {
            room_id: room_id.to_string(),
        })?;

        for conn in room {
            if let Err(error) = conn.send(payload.clone().into()).await {
                tracing::warn!(%error, "failed to write to client");
            }
        }

        Ok(())
    }
}
