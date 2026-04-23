use uuid::Uuid;

use crate::error::AppError;
use crate::models::{UserProfile, UserRegisteredEvent};
use crate::repositories::user_profile_repository::UserProfileRepository;

pub async fn handle_user_registered(
    repository: &UserProfileRepository,
    event: UserRegisteredEvent,
) -> Result<(), AppError> {
    repository.upsert_user_profile(&event).await
}

pub async fn get_user_profile(
    repository: &UserProfileRepository,
    id: Uuid,
) -> Result<UserProfile, AppError> {
    repository
        .find_user_profile_by_id(id)
        .await?
        .ok_or_else(|| AppError::not_found("user profile not found"))
}
