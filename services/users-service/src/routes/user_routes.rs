use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::error::AppError;
use crate::models::UserProfile;
use crate::services::user_service;

pub fn router() -> Router<AppState> {
    Router::new().route("/users/{id}", get(get_user))
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserProfile>, AppError> {
    let profile = user_service::get_user_profile(&state.user_profile_repository, id).await?;

    Ok(Json(profile))
}
