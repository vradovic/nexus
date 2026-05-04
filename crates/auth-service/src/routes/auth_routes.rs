use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::post,
};
use nexus_shared::AppError;

use crate::app_state::AppState;
use crate::models::{LoginRequest, LoginResponse, RegisterRequest, RegisterResponse};
use crate::services::auth_service;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), AppError> {
    let response = auth_service::register_user(&state.auth_repository, &state.nats_client, payload).await?;

    Ok((StatusCode::CREATED, Json(response)))
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let response = auth_service::login_user(&state.auth_repository, &state.jwt_secret, payload).await?;

    Ok(Json(response))
}
