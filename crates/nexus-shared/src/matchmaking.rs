use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const REALTIME_EVENTS_STREAM: &str = "REALTIME_EVENTS";
pub const MATCH_FOUND_SUBJECT: &str = "matchmaking.match_found";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameMode {
    Duel1v1,
}

impl GameMode {
    pub fn required_players(&self) -> usize {
        match self {
            Self::Duel1v1 => 2,
        }
    }

    pub fn redis_queue_key(&self) -> &'static str {
        match self {
            Self::Duel1v1 => "duel_1v_1",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatchFoundEvent {
    pub game_session_id: Uuid,
    pub game_mode: GameMode,
    pub user_ids: Vec<Uuid>,
}
