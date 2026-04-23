use sqlx::PgPool;

use crate::repositories::auth_repository::AuthRepository;

#[derive(Clone)]
pub struct AppState {
    pub auth_repository: AuthRepository,
    pub jwt_secret: String,
}

impl AppState {
    pub fn new(db: PgPool) -> Self {
        let jwt_secret: String =
            std::env::var("JWT_SECRET").expect("JWT_SECRET must be set before starting auth-service");

        Self {
            auth_repository: AuthRepository::new(db),
            jwt_secret,
        }
    }
}
