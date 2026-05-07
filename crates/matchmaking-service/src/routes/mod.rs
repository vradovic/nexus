mod matchmaking_routes;

use axum::{Router, routing::get};

use crate::app_state::AppState;

pub fn app_router(state: AppState) -> Router {
    let matchmaking_routes = matchmaking_routes::router(state.clone());

    Router::new()
        .route("/health", get(health))
        .merge(matchmaking_routes)
        .with_state(state)
}

async fn health() -> &'static str {
    "OK"
}
