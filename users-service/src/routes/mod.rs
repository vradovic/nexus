mod users;

use axum::{Router, routing::get};

async fn health() -> &'static str {
    "OK"
}

pub fn app_router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/users", get(users::get_all_users))
        .route("/users/{id}", get(users::get_single_user))
}
