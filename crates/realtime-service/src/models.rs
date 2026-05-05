use nexus_shared::{GameMode, MatchFoundEvent};
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
        game_session_id: Uuid,
        game_mode: GameMode,
        user_ids: Vec<Uuid>,
    },
}

impl From<MatchFoundEvent> for ServerEvent {
    fn from(event: MatchFoundEvent) -> Self {
        Self::MatchFound {
            game_session_id: event.game_session_id,
            game_mode: event.game_mode,
            user_ids: event.user_ids,
        }
    }
}
