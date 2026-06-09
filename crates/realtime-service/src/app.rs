use axum::{
    Router,
    extract::{WebSocketUpgrade, ws::WebSocket},
    response::Response,
    routing::get,
};
use tower_http::trace::TraceLayer;

pub fn build_router() -> Router {
    Router::new()
        .route("/health", get(handle_health))
        .route("/ws", get(handle_ws))
        .layer(TraceLayer::new_for_http())
}

async fn handle_health() -> &'static str {
    "ok"
}

async fn handle_ws(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    socket
        .send("hello from the moon!".into())
        .await
        .expect("failed to send msg");

    while let Some(result) = socket.recv().await {
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
