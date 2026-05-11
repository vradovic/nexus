use axum::{
    Extension, Json, Router,
    extract::State,
    http::StatusCode,
    middleware,
    routing::{get, post},
};
use nexus_shared::{AppError, AuthenticatedUser};

use crate::{
    app_state::AppState,
    middleware::{require_admin_role, require_player_role},
    models::{
        CreateMatchmakingRuleRequest, JoinMatchmakingRequest, MatchmakingRule,
        MatchmakingStatusResponse, MatchmakingTicket,
    },
    services::matchmaking_service,
};

pub fn router(state: AppState) -> Router<AppState> {
    let player_routes = Router::new()
        .route("/matchmaking/join", post(join_matchmaking))
        .route("/matchmaking/status", get(get_matchmaking_status))
        .route("/matchmaking/leave", post(leave_matchmaking))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_player_role,
        ));

    let admin_routes = Router::new()
        .route("/admin/matchmaking/rules", post(create_matchmaking_rule))
        .route_layer(middleware::from_fn_with_state(state, require_admin_role));

    Router::new().merge(player_routes).merge(admin_routes)
}

async fn join_matchmaking(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<JoinMatchmakingRequest>,
) -> Result<(StatusCode, Json<MatchmakingTicket>), AppError> {
    let ticket = matchmaking_service::join_matchmaking(
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
    let ticket =
        matchmaking_service::get_matchmaking_status(&state.matchmaking_store, user.user_id).await?;

    Ok(Json(MatchmakingStatusResponse { ticket }))
}

async fn leave_matchmaking(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<StatusCode, AppError> {
    matchmaking_service::leave_matchmaking(&state.matchmaking_store, user.user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn create_matchmaking_rule(
    State(state): State<AppState>,
    Json(payload): Json<CreateMatchmakingRuleRequest>,
) -> Result<(StatusCode, Json<MatchmakingRule>), AppError> {
    let rule = matchmaking_service::create_matchmaking_rule(
        &state.matchmaking_rule_repository,
        payload,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(rule)))
}
