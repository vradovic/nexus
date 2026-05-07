use nexus_shared::MatchFoundEvent;
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
        rule_id: Uuid,
        ticket_key: String,
        player_ids: Vec<Uuid>,
    },
}

impl From<MatchFoundEvent> for ServerEvent {
    fn from(event: MatchFoundEvent) -> Self {
        Self::MatchFound {
            rule_id: event.rule_id,
            ticket_key: event.ticket_key,
            player_ids: event.player_ids,
        }
    }
}
