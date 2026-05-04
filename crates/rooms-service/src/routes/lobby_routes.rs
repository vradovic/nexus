use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use nexus_shared::AppError;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::models::{CreateLobbyRequest, Lobby};
use crate::services::lobby_service;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/lobbies", post(create_lobby))
        .route("/lobbies/{id}", get(get_lobby))
}

async fn create_lobby(
    State(state): State<AppState>,
    Json(payload): Json<CreateLobbyRequest>,
) -> Result<(StatusCode, Json<Lobby>), AppError> {
    let lobby = lobby_service::create_lobby(&state.lobby_store, payload).await?;

    Ok((StatusCode::CREATED, Json(lobby)))
}

async fn get_lobby(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Lobby>, AppError> {
    let lobby = lobby_service::get_lobby(&state.lobby_store, id).await?;

    Ok(Json(lobby))
}
