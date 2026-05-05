use std::{collections::HashMap, sync::Arc};

use axum::extract::ws::Message;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::models::ServerEvent;

type UserConnections = HashMap<Uuid, HashMap<Uuid, mpsc::UnboundedSender<Message>>>;

#[derive(Clone)]
pub struct ConnectionRegistry {
    connections: Arc<RwLock<UserConnections>>,
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(
        &self,
        user_id: Uuid,
        connection_id: Uuid,
        sender: mpsc::UnboundedSender<Message>,
    ) {
        let mut connections = self.connections.write().await;
        let user_connections = connections.entry(user_id).or_default();
        user_connections.insert(connection_id, sender);
    }

    pub async fn remove(&self, user_id: Uuid, connection_id: Uuid) {
        let mut connections = self.connections.write().await;

        if let Some(user_connections) = connections.get_mut(&user_id) {
            user_connections.remove(&connection_id);

            if user_connections.is_empty() {
                connections.remove(&user_id);
            }
        }
    }

    pub async fn send_event_to_user(&self, user_id: Uuid, event: &ServerEvent) {
        let payload = match serde_json::to_string(event) {
            Ok(payload) => payload,
            Err(error) => {
                eprintln!("failed to serialize websocket event: {}", error);
                return;
            }
        };

        let senders = {
            let connections = self.connections.read().await;
            connections
                .get(&user_id)
                .map(|user_connections| user_connections.values().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        };

        for sender in senders {
            if sender.send(Message::Text(payload.clone().into())).is_err() {
                eprintln!("failed to send websocket event to user {}", user_id);
            }
        }
    }
}
