use nexus_shared::AppError;
use uuid::Uuid;

use crate::models::{FriendRequest, UserProfile, UserRegisteredEvent};
use crate::repository::UserProfileRepository;

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

pub async fn send_friend_request(
    repository: &UserProfileRepository,
    requester_id: Uuid,
    recipient_id: Uuid,
) -> Result<FriendRequest, AppError> {
    if requester_id == recipient_id {
        return Err(AppError::bad_request(
            "cannot send a friend request to yourself",
        ));
    }

    repository
        .create_friend_request(requester_id, recipient_id)
        .await
}

pub async fn decline_friend_request(
    repository: &UserProfileRepository,
    recipient_id: Uuid,
    request_id: Uuid,
) -> Result<FriendRequest, AppError> {
    repository
        .decline_friend_request(request_id, recipient_id)
        .await?
        .ok_or_else(|| AppError::not_found("pending friend request not found"))
}
