use std::time::{SystemTime, UNIX_EPOCH};

use nexus_shared::AppError;
use uuid::Uuid;

use crate::models::{GameSession, GameSessionPlayer, GameSessionStatus};
use crate::stores::game_session_store::GameSessionStore;

pub async fn create_game_session(
    game_session_store: &GameSessionStore,
    players: Vec<Uuid>,
) -> Result<GameSession, AppError> {
    validate_players(&players)?;

    let game_session = GameSession {
        id: Uuid::new_v4(),
        status: GameSessionStatus::WaitingForStart,
        players: players
            .into_iter()
            .map(|user_id| GameSessionPlayer {
                user_id,
                joined_at: unix_timestamp(),
            })
            .collect(),
    };

    game_session_store.save_game_session(&game_session).await?;

    Ok(game_session)
}

pub async fn get_game_session(
    game_session_store: &GameSessionStore,
    id: Uuid,
) -> Result<GameSession, AppError> {
    game_session_store
        .find_game_session_by_id(id)
        .await?
        .ok_or_else(|| AppError::not_found("game session not found"))
}

fn validate_players(players: &[Uuid]) -> Result<(), AppError> {
    if players.is_empty() {
        return Err(AppError::bad_request(
            "at least one player is required to create a game session",
        ));
    }

    Ok(())
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is invalid")
        .as_secs()
}
