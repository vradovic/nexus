use axum::{Router, routing::get};
use tower_http::trace::TraceLayer;

pub fn build_router() -> Router {
    Router::new()
        .route("/health", get(health))
        .layer(TraceLayer::new_for_http())
}

async fn health() -> &'static str {
    "ok"
}
