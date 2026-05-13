use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatchmakingTicket {
    pub id: Uuid,
    pub player_id: Uuid,
    pub ticket_key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PendingMatch {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub ticket_key: String,
    pub player_ids: Vec<Uuid>,
    pub confirmed_player_ids: Vec<Uuid>,
    pub expires_at_unix_seconds: u64,
}

#[derive(Debug, Serialize, FromRow, Clone)]
pub struct MatchmakingRule {
    pub id: Uuid,
    pub ticket_key: String,
    pub required_players: i32,
}

#[derive(Debug, Deserialize)]
pub struct JoinMatchmakingRequest {
    pub ticket_key: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateMatchmakingRuleRequest {
    pub ticket_key: String,
    pub required_players: i32,
}

#[derive(Debug, Serialize)]
pub struct MatchmakingStatusResponse {
    pub ticket: Option<MatchmakingTicket>,
    pub pending_match: Option<PendingMatch>,
}
