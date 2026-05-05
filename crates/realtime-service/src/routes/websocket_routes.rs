use axum::{
    Router,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use nexus_shared::{AppError, decode_access_token};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    models::{ServerEvent, WebSocketConnectQuery},
};

pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(connect_websocket))
}

async fn connect_websocket(
    State(state): State<AppState>,
    Query(query): Query<WebSocketConnectQuery>,
    websocket_upgrade: WebSocketUpgrade,
) -> Result<Response, AppError> {
    let claims = decode_access_token(&query.token, &state.jwt_secret)?;
    let user_id = claims
        .sub
        .parse::<Uuid>()
        .map_err(|_| AppError::unauthorized("invalid token subject"))?;
    let email = claims.email;

    Ok(websocket_upgrade.on_upgrade(move |socket| {
        handle_socket(state, socket, user_id, email)
    }))
}

async fn handle_socket(
    state: AppState,
    socket: WebSocket,
    user_id: Uuid,
    email: String,
) {
    let connection_id = Uuid::new_v4();
    let (mut sender, mut receiver) = socket.split();
    let (outbound_sender, mut outbound_receiver) = mpsc::unbounded_channel::<Message>();

    state
        .connection_registry
        .register(user_id, connection_id, outbound_sender)
        .await;

    state
        .connection_registry
        .send_event_to_user(
            user_id,
            &ServerEvent::Connected {
                user_id,
                email: email.clone(),
            },
        )
        .await;

    let send_task = tokio::spawn(async move {
        while let Some(message) = outbound_receiver.recv().await {
            if sender.send(message).await.is_err() {
                break;
            }
        }
    });

    while let Some(message_result) = receiver.next().await {
        match message_result {
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(error) => {
                eprintln!("websocket receive error: {}", error);
                break;
            }
        }
    }

    send_task.abort();
    state
        .connection_registry
        .remove(user_id, connection_id)
        .await;
}
