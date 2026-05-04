mod auth_routes;

use axum::{Router, routing::get};

use crate::app_state::AppState;

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .merge(auth_routes::router())
        .with_state(state)
}

async fn health() -> &'static str {
    "OK"
}
