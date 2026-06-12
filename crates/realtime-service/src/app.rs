use std::sync::Arc;

use axum::{
    Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use nexus_shared::nats::NatsAdapter;
use tokio::sync::{Mutex, mpsc};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::messaging::{DEFAULT_ROOM_ID, MessageRouter};

const REALTIME_MESSAGE_EVENT_SUBJECT: &str = "events.realtime.message";

pub struct AppState {
    message_router: Arc<Mutex<MessageRouter>>,
    nats: NatsAdapter,
}

impl AppState {
    pub fn new(nats: NatsAdapter) -> Self {
        Self {
            message_router: Arc::new(Mutex::new(MessageRouter::default())),
            nats,
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

async fn handle_ws(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let conn_id = Uuid::new_v4();
    let (mut socket_sender, mut socket_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    {
        let mut message_router = state.message_router.lock().await;
        message_router
            .add_connection(conn_id, tx, DEFAULT_ROOM_ID)
            .expect("failed to add to default room");
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

    if let Err(error) = state
        .nats
        .publish(REALTIME_MESSAGE_EVENT_SUBJECT.to_string(), payload.into())
        .await
    {
        tracing::error!(%error, %conn_id, "failed to publish client message event");
    }
}
