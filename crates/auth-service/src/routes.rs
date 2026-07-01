use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use nexus_shared::AppError;
use tower_http::cors::CorsLayer;

use crate::app_state::AppState;
use crate::models::{LoginRequest, LoginResponse, RegisterRequest};
use crate::service;

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/register", post(register))
        .route("/login", post(login))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health() -> &'static str {
    "OK"
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<StatusCode, AppError> {
    service::register_user(&state.auth_repository, &state.nats_client, payload).await?;

    Ok(StatusCode::CREATED)
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let response = service::login_user(&state.auth_repository, &state.jwt_secret, payload).await?;

    Ok(Json(response))
}
