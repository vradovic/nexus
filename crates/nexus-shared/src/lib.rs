pub mod auth;
pub mod error;
pub mod matchmaking;
pub mod messaging;

pub use auth::{
    AccessTokenClaims, AuthenticatedUser, UserRole, authenticated_user,
    authenticated_user_from_token, authenticated_user_id, decode_access_token,
};
pub use error::AppError;
pub use matchmaking::{MATCHMAKING_EVENTS_STREAM, MATCH_FOUND_SUBJECT, MatchFoundEvent};
pub use messaging::{ensure_pull_consumer, ensure_stream, publish_json};
