use std::sync::Arc;

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
use nexus_shared::{authenticated_user_from_token, nats::NatsAdapter};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::messaging::{MessageRouter, RoomId};

const REALTIME_MESSAGE_EVENT_SUBJECT: &str = "events.realtime.message";

#[derive(Debug, Default, Deserialize)]
struct WsQuery {
    token: Option<String>,
}

#[derive(Debug, Serialize)]
struct RealtimeMessageEvent {
    connection_id: Uuid,
    rooms: Vec<RoomId>,
    all_rooms: Vec<RoomId>,
    payload: Vec<u8>,
}

pub struct AppState {
    message_router: Arc<Mutex<MessageRouter>>,
    nats: NatsAdapter,
    jwt_secret: String,
}

impl AppState {
    pub fn new(nats: NatsAdapter, jwt_secret: String) -> Self {
        Self {
            message_router: Arc::new(Mutex::new(MessageRouter::default())),
            nats,
            jwt_secret,
        }
    }

    pub fn message_router(&self) -> Arc<Mutex<MessageRouter>> {
        Arc::clone(&self.message_router)
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

    ws.on_upgrade(move |socket| handle_socket(socket, state, user.user_id))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>, conn_id: Uuid) {
    let (mut socket_sender, mut socket_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    {
        let mut message_router = state.message_router.lock().await;
        message_router
            .add_connection(conn_id, tx)
            .expect("failed to add websocket connection to room");
    }
    tracing::debug!(%conn_id, "websocket connection registered");

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

    let (rooms, all_rooms) = {
        let message_router = state.message_router.lock().await;
        let rooms = match message_router.rooms_for_connection(conn_id) {
            Ok(rooms) => rooms,
            Err(error) => {
                tracing::error!(%error, %conn_id, "failed to resolve connection rooms");
                return;
            }
        };
        (rooms, message_router.all_rooms())
    };

    let event = RealtimeMessageEvent {
        connection_id: conn_id,
        rooms,
        all_rooms,
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
