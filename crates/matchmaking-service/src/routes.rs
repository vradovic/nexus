use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    middleware,
    routing::{get, patch, post},
};
use nexus_shared::{AppError, AuthenticatedUser};
use tower_http::cors::CorsLayer;

use crate::{
    app_state::AppState,
    middleware::{require_admin_role, require_player_role},
    models::{
        CreateMatchmakingRuleRequest, JoinMatchmakingRequest, MatchmakingRule,
        MatchmakingStatusResponse, MatchmakingTicket, UpdateMatchmakingRulesRequest,
    },
    service,
};

pub fn app_router(state: AppState) -> Router {
    let player_routes = Router::new()
        .route("/join", post(join_matchmaking))
        .route("/status", get(get_matchmaking_status))
        .route("/leave", post(leave_matchmaking))
        .route("/matches/{match_id}/confirm", post(confirm_match))
        .route("/matches/{match_id}/decline", post(decline_match))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_player_role,
        ));

    let admin_routes = Router::new()
        .route(
            "/admin/matchmaking/rules",
            get(list_matchmaking_rules).post(create_matchmaking_rule),
        )
        .route(
            "/admin/matchmaking/rules/enabled",
            patch(update_matchmaking_rules_enabled),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_admin_role,
        ));

    Router::new()
        .route("/health", get(health))
        .route("/matchmaking/rules", get(list_enabled_matchmaking_rules))
        .merge(player_routes)
        .merge(admin_routes)
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health() -> &'static str {
    "OK"
}

async fn join_matchmaking(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<JoinMatchmakingRequest>,
) -> Result<(StatusCode, Json<MatchmakingTicket>), AppError> {
    let ticket = service::join_matchmaking(
        &state.matchmaking_store,
        &state.matchmaking_rule_repository,
        user.user_id,
        payload,
    )
    .await?;

    Ok((StatusCode::ACCEPTED, Json(ticket)))
}

async fn get_matchmaking_status(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<MatchmakingStatusResponse>, AppError> {
    let (ticket, pending_match) =
        service::get_matchmaking_status(&state.matchmaking_store, user.user_id).await?;

    Ok(Json(MatchmakingStatusResponse {
        ticket,
        pending_match,
    }))
}

async fn list_enabled_matchmaking_rules(
    State(state): State<AppState>,
) -> Result<Json<Vec<MatchmakingRule>>, AppError> {
    let rules = service::list_enabled_matchmaking_rules(&state.matchmaking_rule_repository).await?;

    Ok(Json(rules))
}

async fn leave_matchmaking(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<StatusCode, AppError> {
    service::leave_matchmaking(&state.matchmaking_store, user.user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn confirm_match(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(match_id): Path<uuid::Uuid>,
) -> Result<StatusCode, AppError> {
    service::confirm_match(
        &state.matchmaking_store,
        &state.nats_client,
        user.user_id,
        match_id,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn decline_match(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(match_id): Path<uuid::Uuid>,
) -> Result<StatusCode, AppError> {
    service::decline_match(
        &state.matchmaking_store,
        &state.nats_client,
        user.user_id,
        match_id,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn create_matchmaking_rule(
    State(state): State<AppState>,
    Json(payload): Json<CreateMatchmakingRuleRequest>,
) -> Result<(StatusCode, Json<MatchmakingRule>), AppError> {
    let rule =
        service::create_matchmaking_rule(&state.matchmaking_rule_repository, payload).await?;

    Ok((StatusCode::CREATED, Json(rule)))
}

async fn list_matchmaking_rules(
    State(state): State<AppState>,
) -> Result<Json<Vec<MatchmakingRule>>, AppError> {
    let rules = service::list_matchmaking_rules(&state.matchmaking_rule_repository).await?;

    Ok(Json(rules))
}

async fn update_matchmaking_rules_enabled(
    State(state): State<AppState>,
    Json(payload): Json<UpdateMatchmakingRulesRequest>,
) -> Result<Json<Vec<MatchmakingRule>>, AppError> {
    let rules =
        service::update_matchmaking_rules_enabled(&state.matchmaking_rule_repository, payload)
            .await?;

    Ok(Json(rules))
}
