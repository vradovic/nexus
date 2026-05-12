use sqlx::PgPool;

use crate::repository::UserProfileRepository;

#[derive(Clone)]
pub struct AppState {
    pub user_profile_repository: UserProfileRepository,
}

impl AppState {
    pub fn new(db: PgPool) -> Self {
        Self {
            user_profile_repository: UserProfileRepository::new(db),
        }
    }
}
