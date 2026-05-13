use nexus_shared::{MatchConfirmedEvent, MatchDeclinedEvent, MatchFoundEvent, MatchTimedOutEvent};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct WebSocketConnectQuery {
    pub token: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    Connected { user_id: Uuid, email: String },
    MatchFound {
        match_id: Uuid,
        rule_id: Uuid,
        ticket_key: String,
        player_ids: Vec<Uuid>,
        expires_at_unix_seconds: u64,
    },
    MatchConfirmed {
        match_id: Uuid,
        rule_id: Uuid,
        ticket_key: String,
        player_ids: Vec<Uuid>,
    },
    MatchDeclined {
        match_id: Uuid,
        rule_id: Uuid,
        ticket_key: String,
        player_ids: Vec<Uuid>,
        declined_by: Uuid,
    },
    MatchTimedOut {
        match_id: Uuid,
        rule_id: Uuid,
        ticket_key: String,
        player_ids: Vec<Uuid>,
    },
}

impl From<MatchFoundEvent> for ServerEvent {
    fn from(event: MatchFoundEvent) -> Self {
        Self::MatchFound {
            match_id: event.match_id,
            rule_id: event.rule_id,
            ticket_key: event.ticket_key,
            player_ids: event.player_ids,
            expires_at_unix_seconds: event.expires_at_unix_seconds,
        }
    }
}

impl From<MatchConfirmedEvent> for ServerEvent {
    fn from(event: MatchConfirmedEvent) -> Self {
        Self::MatchConfirmed {
            match_id: event.match_id,
            rule_id: event.rule_id,
            ticket_key: event.ticket_key,
            player_ids: event.player_ids,
        }
    }
}

impl From<MatchDeclinedEvent> for ServerEvent {
    fn from(event: MatchDeclinedEvent) -> Self {
        Self::MatchDeclined {
            match_id: event.match_id,
            rule_id: event.rule_id,
            ticket_key: event.ticket_key,
            player_ids: event.player_ids,
            declined_by: event.declined_by,
        }
    }
}

impl From<MatchTimedOutEvent> for ServerEvent {
    fn from(event: MatchTimedOutEvent) -> Self {
        Self::MatchTimedOut {
            match_id: event.match_id,
            rule_id: event.rule_id,
            ticket_key: event.ticket_key,
            player_ids: event.player_ids,
        }
    }
}
