use sqlx::PgPool;

use crate::repositories::auth_repository::AuthRepository;

#[derive(Clone)]
pub struct AppState {
    pub auth_repository: AuthRepository,
}

impl AppState {
    pub fn new(db: PgPool) -> Self {
        Self {
            auth_repository: AuthRepository::new(db),
        }
    }
}
