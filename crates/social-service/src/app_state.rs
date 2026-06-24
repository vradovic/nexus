use sqlx::PgPool;

use crate::repository::UserProfileRepository;

#[derive(Clone)]
pub struct AppState {
    pub user_profile_repository: UserProfileRepository,
    pub jwt_secret: String,
}

impl AppState {
    pub fn new(db: PgPool, jwt_secret: String) -> Self {
        Self {
            user_profile_repository: UserProfileRepository::new(db),
            jwt_secret,
        }
    }
}
