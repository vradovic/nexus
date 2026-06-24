use axum::{
    Extension, Json, Router,
    extract::{Path, Request, State},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use nexus_shared::{AppError, AuthenticatedUser, authenticated_user};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::models::{
    Friend, FriendRequest, FriendRequestsResponse, SendFriendRequest, UserProfile,
};
use crate::service;

pub fn app_router(state: AppState) -> Router {
    let authenticated_routes = Router::new()
        .route("/friends", get(list_friends))
        .route(
            "/friend-requests",
            get(list_friend_requests).post(send_friend_request),
        )
        .route(
            "/friend-requests/{request_id}/accept",
            post(accept_friend_request),
        )
        .route(
            "/friend-requests/{request_id}/decline",
            post(decline_friend_request),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_authenticated_user,
        ));

    Router::new()
        .route("/health", get(health))
        .route("/users/{id}", get(get_user))
        .merge(authenticated_routes)
        .layer(CorsLayer::permissive())
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

async fn send_friend_request(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<SendFriendRequest>,
) -> Result<Json<FriendRequest>, AppError> {
    let request = service::send_friend_request(
        &state.user_profile_repository,
        user.user_id,
        payload.recipient_id,
    )
    .await?;

    Ok(Json(request))
}

async fn list_friends(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<Friend>>, AppError> {
    let friends = service::list_friends(&state.user_profile_repository, user.user_id).await?;

    Ok(Json(friends))
}

async fn list_friend_requests(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<FriendRequestsResponse>, AppError> {
    let requests =
        service::list_friend_requests(&state.user_profile_repository, user.user_id).await?;

    Ok(Json(requests))
}

async fn accept_friend_request(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(request_id): Path<Uuid>,
) -> Result<Json<FriendRequest>, AppError> {
    let request =
        service::accept_friend_request(&state.user_profile_repository, user.user_id, request_id)
            .await?;

    Ok(Json(request))
}

async fn decline_friend_request(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(request_id): Path<Uuid>,
) -> Result<Json<FriendRequest>, AppError> {
    let request =
        service::decline_friend_request(&state.user_profile_repository, user.user_id, request_id)
            .await?;

    Ok(Json(request))
}

async fn require_authenticated_user(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let user = authenticated_user(request.headers(), &state.jwt_secret)?;
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}
