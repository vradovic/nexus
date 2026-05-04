use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Lobby {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub name: String,
    pub status: LobbyStatus,
    pub members: Vec<LobbyMember>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LobbyMember {
    pub user_id: Uuid,
    pub joined_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum LobbyStatus {
    Open,
    InQueue,
    InMatch,
    Closed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameSession {
    pub id: Uuid,
    pub status: GameSessionStatus,
    pub players: Vec<GameSessionPlayer>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameSessionPlayer {
    pub user_id: Uuid,
    pub joined_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GameSessionStatus {
    WaitingForStart,
    Active,
    Finished,
}

#[derive(Debug, Deserialize)]
pub struct CreateLobbyRequest {
    pub owner_user_id: Uuid,
    pub name: String,
}
