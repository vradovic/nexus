use async_nats::Client as NatsClient;
use redis::Client;

use crate::stores::{
    game_session_store::GameSessionStore, lobby_store::LobbyStore,
    matchmaking_store::MatchmakingStore,
};

#[derive(Clone)]
pub struct AppState {
    pub lobby_store: LobbyStore,
    pub game_session_store: GameSessionStore,
    pub matchmaking_store: MatchmakingStore,
    pub jwt_secret: String,
    pub nats_client: NatsClient,
}

impl AppState {
    pub fn new(redis_client: Client, jwt_secret: String, nats_client: NatsClient) -> Self {
        Self {
            lobby_store: LobbyStore::new(redis_client.clone()),
            game_session_store: GameSessionStore::new(redis_client.clone()),
            matchmaking_store: MatchmakingStore::new(redis_client),
            jwt_secret,
            nats_client,
        }
    }
}
