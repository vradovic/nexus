use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const ACTIVE_USER_PROFILES_REQUEST_SUBJECT: &str = "requests.social.active_user_profiles";

#[derive(Debug, Serialize, Deserialize)]
pub struct ActiveUserProfilesRequest {
    pub user_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActiveUserProfile {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
}
