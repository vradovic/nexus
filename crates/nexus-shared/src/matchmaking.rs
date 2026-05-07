use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MATCHMAKING_EVENTS_STREAM: &str = "MATCHMAKING_EVENTS";
pub const MATCH_FOUND_SUBJECT: &str = "matchmaking.match_found";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatchFoundEvent {
    pub rule_id: Uuid,
    pub ticket_key: String,
    pub player_ids: Vec<Uuid>,
}
