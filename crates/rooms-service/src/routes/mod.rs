mod game_session_routes;
mod lobby_routes;

use axum::{Router, routing::get};

use crate::app_state::AppState;

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .merge(lobby_routes::router())
        .merge(game_session_routes::router())
        .with_state(state)
}

async fn health() -> &'static str {
    "OK"
}
