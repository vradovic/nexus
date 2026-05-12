use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use nexus_shared::AppError;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::models::UserProfile;
use crate::service;

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/users/{id}", get(get_user))
        .with_state(state)
}

async fn health() -> &'static str {
    "OK"
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserProfile>, AppError> {
    let profile = service::get_user_profile(&state.user_profile_repository, id).await?;

    Ok(Json(profile))
}
