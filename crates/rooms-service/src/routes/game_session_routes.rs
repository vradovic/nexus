use axum::{
    Json,
    Router,
    extract::{Path, State},
    routing::get,
};
use nexus_shared::AppError;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::models::GameSession;
use crate::services::game_session_service;

pub fn router() -> Router<AppState> {
    Router::new().route("/game-sessions/{id}", get(get_game_session))
}

async fn get_game_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<GameSession>, AppError> {
    let game_session = game_session_service::get_game_session(&state.game_session_store, id).await?;

    Ok(Json(game_session))
}
