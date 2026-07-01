use axum::{
    Extension, Json, Router,
    extract::{Path, Query, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use nexus_shared::{AppError, AuthenticatedUser, UserRole, authenticated_user};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::models::{
    BlockUserRequest, BlockedUser, ChatMessage, Friend, FriendRequest, FriendRequestsResponse,
    ListChatMessages, SendChatMessage, SendFriendRequest, UserProfile,
};
use crate::{messaging, service};

pub fn app_router(state: AppState) -> Router {
    let authenticated_routes = Router::new()
        .route("/me", get(me))
        .route("/friends", get(list_friends))
        .route(
            "/chat/messages",
            get(list_chat_messages).post(send_chat_message),
        )
        .route("/blocks", get(list_blocks).post(block_user))
        .route("/blocks/{blocked_user_id}", delete(unblock_user))
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

    let admin_routes = Router::new()
        .route("/admin/chat/messages", get(list_all_chat_messages))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_admin_user,
        ));

    Router::new()
        .route("/health", get(health))
        .route("/users/{id}", get(get_user))
        .merge(authenticated_routes)
        .merge(admin_routes)
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health() -> &'static str {
    "OK"
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<UserProfile>, AppError> {
    let user = authenticated_user(&headers, &state.jwt_secret)?;
    let profile = service::get_user_profile(&state.user_profile_repository, user.user_id).await?;

    Ok(Json(profile))
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

async fn send_chat_message(
    State(state): State<AppState>,
    Json(payload): Json<SendChatMessage>,
) -> Result<Json<ChatMessage>, AppError> {
    let message =
        service::send_chat_message(&state.chat_repository, &state.profanity_filter, payload)
            .await?;

    messaging::publish_chat_message(&state.nats_client, &message).await?;

    Ok(Json(message))
}

async fn list_chat_messages(
    State(state): State<AppState>,
    Query(query): Query<ListChatMessages>,
) -> Result<Json<Vec<ChatMessage>>, AppError> {
    let messages =
        service::list_chat_messages(&state.chat_repository, &query.channel, query.limit).await?;

    Ok(Json(messages))
}

async fn list_all_chat_messages(
    State(state): State<AppState>,
) -> Result<Json<Vec<ChatMessage>>, AppError> {
    let messages = service::list_all_chat_messages(&state.chat_repository).await?;

    Ok(Json(messages))
}

async fn list_friends(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<Friend>>, AppError> {
    let friends = service::list_friends(&state.user_profile_repository, user.user_id).await?;

    Ok(Json(friends))
}

async fn block_user(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<BlockUserRequest>,
) -> Result<Json<BlockedUser>, AppError> {
    let blocked = service::block_user(
        &state.user_profile_repository,
        user.user_id,
        payload.blocked_user_id,
    )
    .await?;

    Ok(Json(blocked))
}

async fn unblock_user(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(blocked_user_id): Path<Uuid>,
) -> Result<Response, AppError> {
    service::unblock_user(
        &state.user_profile_repository,
        user.user_id,
        blocked_user_id,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn list_blocks(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<BlockedUser>>, AppError> {
    let blocked_users =
        service::list_blocked_users(&state.user_profile_repository, user.user_id).await?;

    Ok(Json(blocked_users))
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

async fn require_admin_user(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let user = authenticated_user(request.headers(), &state.jwt_secret)?;
    if user.role != UserRole::Admin {
        return Err(AppError::forbidden("admin role is required"));
    }

    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}
