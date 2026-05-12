use async_nats::Client;
use sqlx::PgPool;

use crate::repository::AuthRepository;

#[derive(Clone)]
pub struct AppState {
    pub auth_repository: AuthRepository,
    pub jwt_secret: String,
    pub nats_client: Client,
}

impl AppState {
    pub fn new(db: PgPool, jwt_secret: String, nats_client: Client) -> Self {
        Self {
            auth_repository: AuthRepository::new(db),
            jwt_secret,
            nats_client,
        }
    }
}
