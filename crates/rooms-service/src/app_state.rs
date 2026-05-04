use redis::Client;

use crate::stores::{game_session_store::GameSessionStore, lobby_store::LobbyStore};

#[derive(Clone)]
pub struct AppState {
    pub lobby_store: LobbyStore,
    pub game_session_store: GameSessionStore,
}

impl AppState {
    pub fn new(redis_client: Client) -> Self {
        Self {
            lobby_store: LobbyStore::new(redis_client.clone()),
            game_session_store: GameSessionStore::new(redis_client),
        }
    }
}
