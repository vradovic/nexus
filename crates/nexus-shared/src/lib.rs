pub mod auth;
pub mod error;
pub mod matchmaking;
pub mod messaging;
pub mod nats;
pub mod redis_store;

pub use auth::{
    AccessTokenClaims, AuthenticatedUser, UserRole, authenticated_user,
    authenticated_user_from_token, authenticated_user_id, decode_access_token,
};
pub use error::AppError;
pub use matchmaking::{
    MATCH_CHANNEL_READY_SUBJECT, MATCH_CONFIRMED_SUBJECT, MATCH_DECLINED_SUBJECT,
    MATCH_FOUND_SUBJECT, MATCH_TIMED_OUT_SUBJECT, MatchChannelReadyEvent, MatchConfirmedEvent,
    MatchDeclinedEvent, MatchFoundEvent, MatchTimedOutEvent,
};
pub use messaging::{ensure_pull_consumer, ensure_stream, publish_json};
pub use redis_store::{read_json, write_json};
