mod app_state;
mod db;
mod messaging;
mod models;
mod profanity;
mod repository;
mod routes;
mod service;

use std::net::SocketAddr;

use app_state::AppState;
use axum::Router;
use db::init_db;
use messaging::{
    ensure_commands_stream, ensure_events_stream, ensure_registration_consumer,
    start_active_user_profiles_responder, start_user_registered_consumer,
};
use profanity::ProfanityFilter;

const DEFAULT_BAD_WORDS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/bad_words.txt");

#[tokio::main]
async fn main() {
    load_env();

    let pool = init_db().await;
    let nats_url =
        std::env::var("NATS_URL").expect("NATS_URL must be set before starting social-service");
    let jwt_secret =
        std::env::var("JWT_SECRET").expect("JWT_SECRET must be set before starting social-service");
    let nats_client = async_nats::connect(nats_url)
        .await
        .expect("failed to connect to nats");
    ensure_events_stream(&nats_client).await;
    ensure_commands_stream(&nats_client).await;
    ensure_registration_consumer(&nats_client).await;

    let bad_words_path =
        std::env::var("BAD_WORDS_PATH").unwrap_or_else(|_| DEFAULT_BAD_WORDS_PATH.to_string());
    let profanity_filter =
        ProfanityFilter::from_file(&bad_words_path).expect("failed to load bad words file");
    let state = AppState::new(pool, nats_client.clone(), jwt_secret, profanity_filter);
    let registration_nats_client = nats_client.clone();
    let active_users_nats_client = nats_client.clone();
    let consumer_repository = state.user_profile_repository.clone();
    let active_users_repository = state.user_profile_repository.clone();

    tokio::spawn(async move {
        start_user_registered_consumer(registration_nats_client, consumer_repository).await;
    });
    tokio::spawn(async move {
        start_active_user_profiles_responder(active_users_nats_client, active_users_repository)
            .await;
    });

    let app: Router = routes::app_router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3002));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    println!("social-service listening on http://{}", addr);

    axum::serve(listener, app).await.expect("server failed");
}

fn load_env() {
    dotenvy::dotenv().ok();
    dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).ok();
}
