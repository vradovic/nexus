use async_nats::Client;
use sqlx::PgPool;

use crate::profanity::ProfanityFilter;
use crate::repository::{ChatRepository, UserProfileRepository};

#[derive(Clone)]
pub struct AppState {
    pub user_profile_repository: UserProfileRepository,
    pub chat_repository: ChatRepository,
    pub nats_client: Client,
    pub profanity_filter: ProfanityFilter,
    pub jwt_secret: String,
}

impl AppState {
    pub fn new(
        db: PgPool,
        nats_client: Client,
        jwt_secret: String,
        profanity_filter: ProfanityFilter,
    ) -> Self {
        Self {
            user_profile_repository: UserProfileRepository::new(db.clone()),
            chat_repository: ChatRepository::new(db),
            nats_client,
            profanity_filter,
            jwt_secret,
        }
    }
}
