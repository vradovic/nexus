pub mod auth;
pub mod error;
pub mod matchmaking;
pub mod messaging;

pub use auth::{AccessTokenClaims, authenticated_user_id, decode_access_token};
pub use error::AppError;
pub use matchmaking::{GameMode, MATCH_FOUND_SUBJECT, MatchFoundEvent, REALTIME_EVENTS_STREAM};
pub use messaging::{ensure_pull_consumer, ensure_stream, publish_json};
