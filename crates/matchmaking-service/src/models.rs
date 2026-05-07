use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatchmakingTicket {
    pub id: Uuid,
    pub player_id: Uuid,
    pub ticket_key: String,
}

#[derive(Debug, FromRow, Clone)]
pub struct MatchmakingRule {
    pub id: Uuid,
    pub ticket_key: String,
    pub required_players: i32,
}

#[derive(Debug, Deserialize)]
pub struct JoinMatchmakingRequest {
    pub ticket_key: String,
}

#[derive(Debug, Serialize)]
pub struct MatchmakingStatusResponse {
    pub ticket: Option<MatchmakingTicket>,
}
