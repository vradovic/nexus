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
use tokio::sync::{Mutex, mpsc};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::messaging::{DEFAULT_ROOM_ID, MessageRouter};

#[derive(Default)]
pub struct AppState {
    message_router: Mutex<MessageRouter>,
}

pub fn build_router() -> Router {
    Router::new()
        .route("/health", get(handle_health))
        .route("/ws", get(handle_ws))
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(AppState::default()))
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

        tracing::debug!(?msg, "received websocket message");
    }

    {
        let mut message_router = state.message_router.lock().await;
        message_router.remove_connection(conn_id);
    }
    writer_task.abort();
    tracing::debug!(%conn_id, "websocket connection removed");
}
