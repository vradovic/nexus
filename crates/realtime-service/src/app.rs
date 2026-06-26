use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use axum::{
    Router,
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use nexus_shared::{
    MATCH_CHANNEL_READY_SUBJECT, MatchChannelReadyEvent, MatchConfirmedEvent,
    authenticated_user_from_token,
    nats::{NatsAdapter, NatsError},
};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::messaging::{ChannelId, MessageRouter, MessagingError};

const REALTIME_MESSAGE_EVENT_SUBJECT: &str = "events.realtime.message";

fn user_channel_id(user_id: Uuid) -> ChannelId {
    format!("user:{user_id}")
}

#[derive(Debug, Default, Deserialize)]
struct WsQuery {
    token: Option<String>,
}

#[derive(Debug, Serialize)]
struct RealtimeMessageEvent {
    connection_id: Uuid,
    channels: Vec<ChannelId>,
    all_channels: Vec<ChannelId>,
    payload: Vec<u8>,
}

pub struct AppState {
    message_router: Arc<Mutex<MessageRouter>>,
    match_channels: Arc<Mutex<MatchChannelRegistry>>,
    nats: NatsAdapter,
    jwt_secret: String,
}

#[derive(Debug, Default)]
struct MatchChannelRegistry {
    pending_match_channels: HashMap<Uuid, PendingMatchChannel>,
}

#[derive(Debug)]
struct PendingMatchChannel {
    ready_event: MatchChannelReadyEvent,
    joined_player_ids: HashSet<Uuid>,
}

impl AppState {
    pub fn new(nats: NatsAdapter, jwt_secret: String) -> Self {
        Self {
            message_router: Arc::new(Mutex::new(MessageRouter::default())),
            match_channels: Arc::new(Mutex::new(MatchChannelRegistry::default())),
            nats,
            jwt_secret,
        }
    }

    pub fn message_router(&self) -> Arc<Mutex<MessageRouter>> {
        Arc::clone(&self.message_router)
    }

    pub async fn publish_json<T>(&self, subject: &str, payload: &T) -> Result<(), NatsError>
    where
        T: Serialize,
    {
        self.nats.publish_json(subject, payload).await
    }

    pub async fn create_match_channel(
        &self,
        event: MatchConfirmedEvent,
    ) -> Result<Option<MatchChannelReadyEvent>, MessagingError> {
        let mut message_router = self.message_router.lock().await;
        let mut match_channels = self.match_channels.lock().await;

        match_channels.create_match_channel(&mut message_router, event)
    }

    pub async fn join_pending_match_channels(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<MatchChannelReadyEvent>, MessagingError> {
        let mut message_router = self.message_router.lock().await;
        let mut match_channels = self.match_channels.lock().await;

        match_channels.join_pending_match_channels(&mut message_router, player_id)
    }

    pub async fn has_active_connection(&self, user_id: Uuid) -> bool {
        self.message_router.lock().await.has_connection(user_id)
    }
}

impl MatchChannelRegistry {
    fn create_match_channel(
        &mut self,
        message_router: &mut MessageRouter,
        event: MatchConfirmedEvent,
    ) -> Result<Option<MatchChannelReadyEvent>, MessagingError> {
        let ready_event = MatchChannelReadyEvent {
            match_id: event.match_id,
            rule_id: event.rule_id,
            ticket_key: event.ticket_key,
            player_ids: event.player_ids,
            channel: format!("match:{}", event.match_id),
        };

        let mut joined_player_ids = HashSet::<Uuid>::new();

        message_router.create_channel(&ready_event.channel);

        for player_id in &ready_event.player_ids {
            match message_router.join_channel(*player_id, &ready_event.channel) {
                Ok(()) => {
                    joined_player_ids.insert(*player_id);
                }
                Err(MessagingError::ConnectionNotFound { conn_id }) => {
                    tracing::warn!(
                        %conn_id,
                        channel = %ready_event.channel,
                        "matched player does not have an active realtime connection yet"
                    );
                }
                Err(error) => return Err(error),
            }
        }

        if joined_player_ids.len() == ready_event.player_ids.len() {
            return Ok(Some(ready_event));
        }

        self.pending_match_channels.insert(
            ready_event.match_id,
            PendingMatchChannel {
                ready_event,
                joined_player_ids,
            },
        );

        Ok(None)
    }

    fn join_pending_match_channels(
        &mut self,
        message_router: &mut MessageRouter,
        player_id: Uuid,
    ) -> Result<Vec<MatchChannelReadyEvent>, MessagingError> {
        let mut ready_events = Vec::<MatchChannelReadyEvent>::new();
        let mut completed_match_ids = Vec::<Uuid>::new();

        for (match_id, pending) in self.pending_match_channels.iter_mut() {
            if !pending.ready_event.player_ids.contains(&player_id) {
                continue;
            }

            match message_router.join_channel(player_id, &pending.ready_event.channel) {
                Ok(()) => {
                    pending.joined_player_ids.insert(player_id);
                }
                Err(MessagingError::ConnectionNotFound { conn_id }) => {
                    tracing::warn!(
                        %conn_id,
                        channel = %pending.ready_event.channel,
                        "matched player disconnected before pending channel join"
                    );
                    continue;
                }
                Err(error) => return Err(error),
            }

            if pending.joined_player_ids.len() == pending.ready_event.player_ids.len() {
                ready_events.push(pending.ready_event.clone());
                completed_match_ids.push(*match_id);
            }
        }

        for match_id in completed_match_ids {
            self.pending_match_channels.remove(&match_id);
        }

        Ok(ready_events)
    }
}

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(handle_health))
        .route("/ws", get(handle_ws))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn handle_health() -> &'static str {
    "ok"
}

async fn handle_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsQuery>,
) -> Response {
    let Some(token) = query.token.as_deref() else {
        return (StatusCode::UNAUTHORIZED, "missing access token").into_response();
    };

    let user = match authenticated_user_from_token(token, &state.jwt_secret) {
        Ok(user) => user,
        Err(error) => {
            tracing::warn!(?error, "websocket authentication failed");
            return (StatusCode::UNAUTHORIZED, "invalid access token").into_response();
        }
    };

    tracing::debug!(
        user_id = %user.user_id,
        email = %user.email,
        role = %user.role,
        "websocket authenticated"
    );

    if state.has_active_connection(user.user_id).await {
        return (
            StatusCode::CONFLICT,
            "user already has an active websocket connection",
        )
            .into_response();
    }

    ws.on_upgrade(move |socket| handle_socket(socket, state, user.user_id))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>, conn_id: Uuid) {
    let (mut socket_sender, mut socket_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let user_channel = user_channel_id(conn_id);

    {
        let mut message_router = state.message_router.lock().await;
        if let Err(error) = message_router.add_connection(conn_id, tx) {
            tracing::warn!(%error, %conn_id, "failed to add websocket connection");
            return;
        }
        message_router.create_channel(&user_channel);
        if let Err(error) = message_router.join_channel(conn_id, &user_channel) {
            message_router.remove_connection(conn_id);
            tracing::error!(
                %error,
                %conn_id,
                channel = %user_channel,
                "failed to join user channel"
            );
            return;
        }
    }
    tracing::debug!(%conn_id, "websocket connection registered");

    match state.join_pending_match_channels(conn_id).await {
        Ok(ready_events) => {
            for ready_event in ready_events {
                if let Err(error) = state
                    .publish_json(MATCH_CHANNEL_READY_SUBJECT, &ready_event)
                    .await
                {
                    tracing::error!(
                        %error,
                        match_id = %ready_event.match_id,
                        channel = %ready_event.channel,
                        "failed to publish pending match channel ready event"
                    );
                }
            }
        }
        Err(error) => {
            tracing::error!(%error, %conn_id, "failed to join pending match channels");
        }
    }

    let writer_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if let Err(error) = socket_sender.send(message).await {
                tracing::error!(%error, "failed to send websocket message");
                break;
            }
        }
    });

    while let Some(result) = socket_receiver.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(error) => {
                tracing::error!(%error, "connection closed");
                break;
            }
        };

        publish_client_message(conn_id, &state, &msg).await;
        tracing::debug!(?msg, "received websocket message");
    }

    {
        let mut message_router = state.message_router.lock().await;
        message_router.remove_connection(conn_id);
        if let Err(error) = message_router.remove_channel(&user_channel) {
            tracing::error!(%error, %conn_id, channel = %user_channel, "failed to remove user channel");
        }
    }
    writer_task.abort();
    tracing::debug!(%conn_id, "websocket connection removed");
}

async fn publish_client_message(conn_id: Uuid, state: &AppState, msg: &Message) {
    let payload = match msg {
        Message::Text(text) => text.as_str().as_bytes().to_vec(),
        Message::Binary(bytes) => bytes.to_vec(),
        Message::Close(_) | Message::Ping(_) | Message::Pong(_) => return,
    };

    let (channels, all_channels) = {
        let message_router = state.message_router.lock().await;
        let channels = match message_router.channels_for_connection(conn_id) {
            Ok(channels) => channels,
            Err(error) => {
                tracing::error!(%error, %conn_id, "failed to resolve connection channels");
                return;
            }
        };
        (channels, message_router.all_channels())
    };

    let event = RealtimeMessageEvent {
        connection_id: conn_id,
        channels,
        all_channels,
        payload,
    };

    if let Err(error) = state
        .nats
        .publish_json(REALTIME_MESSAGE_EVENT_SUBJECT, &event)
        .await
    {
        tracing::error!(%error, %conn_id, "failed to publish client message event");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_channel_is_ready_when_all_players_are_connected() {
        let mut registry = MatchChannelRegistry::default();
        let mut router = MessageRouter::default();
        let match_id = Uuid::new_v4();
        let rule_id = Uuid::new_v4();
        let alpha = Uuid::new_v4();
        let beta = Uuid::new_v4();
        let (alpha_tx, _alpha_rx) = mpsc::unbounded_channel::<Message>();
        let (beta_tx, _beta_rx) = mpsc::unbounded_channel::<Message>();

        router
            .add_connection(alpha, alpha_tx)
            .expect("alpha connection is added");
        router
            .add_connection(beta, beta_tx)
            .expect("beta connection is added");

        let ready_event = registry
            .create_match_channel(
                &mut router,
                MatchConfirmedEvent {
                    match_id,
                    rule_id,
                    ticket_key: "duel".to_string(),
                    player_ids: vec![alpha, beta],
                },
            )
            .expect("match channel is created")
            .expect("match channel is ready");

        let channel = format!("match:{match_id}");
        assert_eq!(ready_event.channel, channel);
        assert_eq!(ready_event.match_id, match_id);
        assert_eq!(ready_event.rule_id, rule_id);
        assert_eq!(ready_event.ticket_key, "duel");
        assert_eq!(ready_event.player_ids, vec![alpha, beta]);
        assert_eq!(
            router
                .channels_for_connection(alpha)
                .expect("alpha channels"),
            vec!["default".to_string(), channel.clone()]
        );
        assert_eq!(
            router.channels_for_connection(beta).expect("beta channels"),
            vec!["default".to_string(), channel]
        );
    }

    #[test]
    fn match_channel_waits_until_missing_player_connects() {
        let mut registry = MatchChannelRegistry::default();
        let mut router = MessageRouter::default();
        let match_id = Uuid::new_v4();
        let rule_id = Uuid::new_v4();
        let alpha = Uuid::new_v4();
        let beta = Uuid::new_v4();
        let (alpha_tx, _alpha_rx) = mpsc::unbounded_channel::<Message>();

        router
            .add_connection(alpha, alpha_tx)
            .expect("alpha connection is added");

        let ready_event = registry
            .create_match_channel(
                &mut router,
                MatchConfirmedEvent {
                    match_id,
                    rule_id,
                    ticket_key: "duel".to_string(),
                    player_ids: vec![alpha, beta],
                },
            )
            .expect("match channel is created");

        assert!(ready_event.is_none());
        assert_eq!(registry.pending_match_channels.len(), 1);

        let channel = format!("match:{match_id}");
        assert_eq!(
            router
                .channels_for_connection(alpha)
                .expect("alpha channels"),
            vec!["default".to_string(), channel.clone()]
        );
        assert!(router.channels_for_connection(beta).is_err());

        let (beta_tx, _beta_rx) = mpsc::unbounded_channel::<Message>();
        router
            .add_connection(beta, beta_tx)
            .expect("beta connection is added");

        let ready_events = registry
            .join_pending_match_channels(&mut router, beta)
            .expect("pending match channel is joined");

        assert_eq!(ready_events.len(), 1);
        assert_eq!(ready_events[0].match_id, match_id);
        assert_eq!(ready_events[0].channel, channel);
        assert!(registry.pending_match_channels.is_empty());
        assert_eq!(
            router.channels_for_connection(beta).expect("beta channels"),
            vec!["default".to_string(), format!("match:{match_id}")]
        );
    }
}
