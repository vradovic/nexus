use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use nexus_shared::{AppError, AuthenticatedUser, UserRole, authenticated_user};

use crate::app_state::AppState;

pub async fn require_player_role(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let user = authenticated_user(request.headers(), &state.jwt_secret)?;

    if user.role != UserRole::Player {
        return Err(AppError::forbidden("player role is required"));
    }

    request.extensions_mut().insert::<AuthenticatedUser>(user);

    Ok(next.run(request).await)
}
