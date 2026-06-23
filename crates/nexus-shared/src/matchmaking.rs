use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MATCH_FOUND_SUBJECT: &str = "events.matchmaking.match_found";
pub const MATCH_CONFIRMED_SUBJECT: &str = "events.matchmaking.match_confirmed";
pub const MATCH_DECLINED_SUBJECT: &str = "events.matchmaking.match_declined";
pub const MATCH_TIMED_OUT_SUBJECT: &str = "events.matchmaking.match_timed_out";
pub const MATCH_CHANNEL_READY_SUBJECT: &str = "events.realtime.match_channel_ready";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatchFoundEvent {
    pub match_id: Uuid,
    pub rule_id: Uuid,
    pub ticket_key: String,
    pub player_ids: Vec<Uuid>,
    pub expires_at_unix_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatchConfirmedEvent {
    pub match_id: Uuid,
    pub rule_id: Uuid,
    pub ticket_key: String,
    pub player_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatchChannelReadyEvent {
    pub match_id: Uuid,
    pub rule_id: Uuid,
    pub ticket_key: String,
    pub player_ids: Vec<Uuid>,
    pub channel: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatchDeclinedEvent {
    pub match_id: Uuid,
    pub rule_id: Uuid,
    pub ticket_key: String,
    pub player_ids: Vec<Uuid>,
    pub declined_by: Uuid,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatchTimedOutEvent {
    pub match_id: Uuid,
    pub rule_id: Uuid,
    pub ticket_key: String,
    pub player_ids: Vec<Uuid>,
}
