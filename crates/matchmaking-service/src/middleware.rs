use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use nexus_shared::{AppError, AuthenticatedUser, UserRole, authenticated_user};

use crate::app_state::AppState;

pub async fn require_player_role(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    require_role(
        state,
        request,
        next,
        UserRole::Player,
        "player role is required",
    )
    .await
}

pub async fn require_admin_role(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    require_role(
        state,
        request,
        next,
        UserRole::Admin,
        "admin role is required",
    )
    .await
}

async fn require_role(
    state: AppState,
    mut request: Request,
    next: Next,
    required_role: UserRole,
    forbidden_message: &str,
) -> Result<Response, AppError> {
    let user = authenticated_user(request.headers(), &state.jwt_secret)?;

    if user.role != required_role {
        return Err(AppError::forbidden(forbidden_message));
    }

    request.extensions_mut().insert::<AuthenticatedUser>(user);

    Ok(next.run(request).await)
}
