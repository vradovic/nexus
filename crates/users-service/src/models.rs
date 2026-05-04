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
