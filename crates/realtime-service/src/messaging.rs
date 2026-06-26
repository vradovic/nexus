use std::collections::{HashMap, HashSet};

use axum::extract::ws::Message;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

pub const DEFAULT_CHANNEL_ID: &str = "default";

pub type Sender = UnboundedSender<Message>;
pub type ChannelId = String;
pub type ConnectionId = Uuid;

#[derive(Debug)]
pub struct MessageRouter {
    connections: HashMap<ConnectionId, Sender>,
    channels: HashMap<ChannelId, HashSet<ConnectionId>>,
    connection_channels: HashMap<ConnectionId, HashSet<ChannelId>>,
}

#[derive(Debug, Error)]
pub enum MessagingError {
    #[error("connection {conn_id} not found")]
    ConnectionNotFound { conn_id: ConnectionId },
    #[error("connection {conn_id} already exists")]
    ConnectionAlreadyExists { conn_id: ConnectionId },
    #[error("channel {channel_id} not found")]
    ChannelNotFound { channel_id: ChannelId },
    #[error("default channel cannot be removed")]
    CannotRemoveDefaultChannel,
}

impl MessageRouter {
    pub fn add_connection(
        &mut self,
        conn_id: ConnectionId,
        tx: Sender,
    ) -> Result<(), MessagingError> {
        if self.connections.contains_key(&conn_id) {
            return Err(MessagingError::ConnectionAlreadyExists { conn_id });
        }

        self.connections.insert(conn_id, tx);
        if let Err(error) = self.join_channel(conn_id, DEFAULT_CHANNEL_ID) {
            self.connections.remove(&conn_id);
            return Err(error);
        }

        Ok(())
    }

    pub fn has_connection(&self, conn_id: ConnectionId) -> bool {
        self.connections.contains_key(&conn_id)
    }

    pub fn remove_connection(&mut self, conn_id: ConnectionId) {
        self.connections.remove(&conn_id);

        let Some(channel_ids) = self.connection_channels.remove(&conn_id) else {
            return;
        };

        for channel_id in channel_ids {
            if let Some(channel) = self.channels.get_mut(&channel_id) {
                channel.remove(&conn_id);
            }
        }
    }

    pub fn broadcast(&mut self, message: Message) {
        let conn_ids = self.connections.keys().copied().collect::<Vec<_>>();
        self.send_to_connections(conn_ids, message);
    }

    pub fn broadcast_to_channels(&mut self, channel_ids: &[ChannelId], message: Message) {
        let mut conn_ids = HashSet::<ConnectionId>::new();

        for channel_id in channel_ids {
            if let Some(channel) = self.channels.get(channel_id) {
                conn_ids.extend(channel.iter().copied());
            }
        }

        self.send_to_connections(conn_ids.into_iter().collect(), message);
    }

    pub fn create_channel(&mut self, channel_id: &str) {
        self.channels.entry(channel_id.to_string()).or_default();
    }

    pub fn remove_channel(&mut self, channel_id: &str) -> Result<(), MessagingError> {
        if channel_id == DEFAULT_CHANNEL_ID {
            return Err(MessagingError::CannotRemoveDefaultChannel);
        }

        let Some(conn_ids) = self.channels.remove(channel_id) else {
            return Ok(());
        };

        for conn_id in conn_ids {
            if let Some(channel_ids) = self.connection_channels.get_mut(&conn_id) {
                channel_ids.remove(channel_id);
            }
        }

        Ok(())
    }

    pub fn all_channels(&self) -> Vec<ChannelId> {
        let mut channel_ids = self.channels.keys().cloned().collect::<Vec<_>>();
        channel_ids.sort();
        channel_ids
    }

    pub fn channels_for_connection(
        &self,
        conn_id: ConnectionId,
    ) -> Result<Vec<ChannelId>, MessagingError> {
        let Some(channel_ids) = self.connection_channels.get(&conn_id) else {
            return Err(MessagingError::ConnectionNotFound { conn_id });
        };

        let mut channel_ids = channel_ids.iter().cloned().collect::<Vec<_>>();
        channel_ids.sort();
        Ok(channel_ids)
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

    pub fn join_channel(
        &mut self,
        conn_id: ConnectionId,
        channel_id: &str,
    ) -> Result<(), MessagingError> {
        if !self.connections.contains_key(&conn_id) {
            return Err(MessagingError::ConnectionNotFound { conn_id });
        }

        let Some(channel) = self.channels.get_mut(channel_id) else {
            return Err(MessagingError::ChannelNotFound {
                channel_id: channel_id.to_string(),
            });
        };

        channel.insert(conn_id);

        self.connection_channels
            .entry(conn_id)
            .or_default()
            .insert(channel_id.to_string());

        Ok(())
    }

    pub fn leave_channel(
        &mut self,
        conn_id: ConnectionId,
        channel_id: &str,
    ) -> Result<(), MessagingError> {
        if !self.connections.contains_key(&conn_id) {
            return Err(MessagingError::ConnectionNotFound { conn_id });
        }

        let Some(channel) = self.channels.get_mut(channel_id) else {
            return Err(MessagingError::ChannelNotFound {
                channel_id: channel_id.to_string(),
            });
        };

        channel.remove(&conn_id);

        if let Some(channel_ids) = self.connection_channels.get_mut(&conn_id) {
            channel_ids.remove(channel_id);
        }

        Ok(())
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        let mut channels = HashMap::<ChannelId, HashSet<ConnectionId>>::new();
        channels.insert(
            DEFAULT_CHANNEL_ID.to_string(),
            HashSet::<ConnectionId>::new(),
        );

        Self {
            channels,
            connections: Default::default(),
            connection_channels: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::sync::mpsc;

    #[test]
    fn broadcast_to_channels_only_sends_to_channel_members() {
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
        router.create_channel("alpha");
        router
            .join_channel(alpha_conn, "alpha")
            .expect("alpha connection joins alpha channel");

        router.broadcast_to_channels(&["alpha".to_string()], Message::Text("hello".into()));

        match alpha_rx.try_recv().expect("alpha receives message") {
            Message::Text(text) => assert_eq!(text.as_str(), "hello"),
            message => panic!("unexpected message: {message:?}"),
        }
        assert!(beta_rx.try_recv().is_err());
    }

    #[test]
    fn channel_lifecycle_updates_channel_membership_indexes() {
        let mut router = MessageRouter::default();
        let conn_id = Uuid::new_v4();
        let (tx, _rx) = mpsc::unbounded_channel();

        router
            .add_connection(conn_id, tx)
            .expect("connection is added");
        router.create_channel("match-1");
        router
            .join_channel(conn_id, "match-1")
            .expect("connection joins channel");

        assert_eq!(
            router
                .channels_for_connection(conn_id)
                .expect("channels exist"),
            vec!["default".to_string(), "match-1".to_string()]
        );
        assert_eq!(
            router.all_channels(),
            vec!["default".to_string(), "match-1".to_string()]
        );

        router
            .leave_channel(conn_id, "match-1")
            .expect("connection leaves channel");
        assert_eq!(
            router
                .channels_for_connection(conn_id)
                .expect("channels exist"),
            vec!["default".to_string()]
        );

        router
            .join_channel(conn_id, "match-1")
            .expect("connection rejoins channel");
        router
            .remove_channel("match-1")
            .expect("channel is removed");
        assert_eq!(
            router
                .channels_for_connection(conn_id)
                .expect("channels exist"),
            vec!["default".to_string()]
        );
        assert_eq!(router.all_channels(), vec!["default".to_string()]);
    }
}
