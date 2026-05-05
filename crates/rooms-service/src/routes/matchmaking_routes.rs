use axum::{
    Json, Router,
    extract::{State},
    http::{HeaderMap, StatusCode},
    routing::post,
};
use nexus_shared::{AppError, auth::authenticated_user_id};

use crate::{
    app_state::AppState,
    models::{JoinMatchmakingRequest, MatchmakingTicket},
    services::matchmaking_service,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/matchmaking/join", post(join_matchmaking))
}

async fn join_matchmaking(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<JoinMatchmakingRequest>,
) -> Result<(StatusCode, Json<MatchmakingTicket>), AppError> {
    let user_id = authenticated_user_id(&headers, &state.jwt_secret)?;
    let ticket =
        matchmaking_service::join_matchmaking(&state.matchmaking_store, user_id, payload).await?;

    Ok((StatusCode::ACCEPTED, Json(ticket)))
}
