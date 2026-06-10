use std::sync::{Arc, RwLock};

use axum::{
    Router,
    extract::{WebSocketUpgrade, ws::WebSocket},
    response::Response,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use tower_http::trace::TraceLayer;

use crate::rooms::RoomRegistry;

#[derive(Default)]
pub struct AppState {
    room_registry: RwLock<RoomRegistry>,
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

async fn handle_ws(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    let (mut sink, mut stream) = socket.split();

    sink.send("hello from the moon!".into())
        .await
        .expect("failed to send msg");

    while let Some(result) = stream.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(error) => {
                tracing::error!(%error, "connection closed");
                return;
            }
        };

        tracing::debug!(?msg, "received websocket message");
    }
}
