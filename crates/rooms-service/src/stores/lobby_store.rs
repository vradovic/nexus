use nexus_shared::AppError;
use redis::Client;
use uuid::Uuid;

use crate::models::Lobby;
use crate::redis_store;

#[derive(Clone)]
pub struct LobbyStore {
    redis_client: Client,
}

impl LobbyStore {
    pub fn new(redis_client: Client) -> Self {
        Self { redis_client }
    }

    pub async fn save_lobby(&self, lobby: &Lobby) -> Result<(), AppError> {
        let key = lobby_key(lobby.id);

        redis_store::write_json(&self.redis_client, &key, lobby).await
    }

    pub async fn find_lobby_by_id(&self, id: Uuid) -> Result<Option<Lobby>, AppError> {
        let key = lobby_key(id);

        redis_store::read_json(&self.redis_client, &key).await
    }
}

fn lobby_key(id: Uuid) -> String {
    format!("lobbies:{id}")
}
