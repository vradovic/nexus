use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow)]
pub struct UserProfile {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
}

#[derive(Debug, Deserialize)]
pub struct UserRegisteredEvent {
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
}

#[derive(Debug, Deserialize)]
pub struct SendFriendRequest {
    pub recipient_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct BlockUserRequest {
    pub blocked_user_id: Uuid,
}

#[derive(Debug, Serialize, FromRow)]
pub struct FriendRequest {
    pub id: Uuid,
    pub requester_id: Uuid,
    pub recipient_id: Uuid,
    pub status: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct FriendRequestView {
    pub id: Uuid,
    pub requester_id: Uuid,
    pub requester_first_name: String,
    pub requester_last_name: String,
    pub recipient_id: Uuid,
    pub recipient_first_name: String,
    pub recipient_last_name: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct FriendRequestsResponse {
    pub incoming: Vec<FriendRequestView>,
    pub outgoing: Vec<FriendRequestView>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Friend {
    pub friendship_id: Uuid,
    pub friend_id: Uuid,
    pub first_name: String,
    pub last_name: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct BlockedUser {
    pub block_id: Uuid,
    pub blocked_user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
}

#[derive(Debug, Deserialize)]
pub struct SendChatMessage {
    pub channel: String,
    #[serde(alias = "sender")]
    pub sender_id: Uuid,
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct ListChatMessages {
    pub channel: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ChatMessage {
    pub id: Uuid,
    pub channel: String,
    pub sender_id: Uuid,
    pub body: String,
    pub created_at: String,
}
