use nexus_shared::AppError;
use redis::Client;
use uuid::Uuid;

use crate::models::GameSession;
use crate::redis_store;

#[derive(Clone)]
pub struct GameSessionStore {
    redis_client: Client,
}

impl GameSessionStore {
    pub fn new(redis_client: Client) -> Self {
        Self { redis_client }
    }

    pub async fn save_game_session(&self, game_session: &GameSession) -> Result<(), AppError> {
        let key = game_session_key(game_session.id);

        redis_store::write_json(&self.redis_client, &key, game_session).await
    }

    pub async fn find_game_session_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<GameSession>, AppError> {
        let key = game_session_key(id);

        redis_store::read_json(&self.redis_client, &key).await
    }
}

fn game_session_key(id: Uuid) -> String {
    format!("game_sessions:{id}")
}
