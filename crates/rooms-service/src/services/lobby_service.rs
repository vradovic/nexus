use std::time::{SystemTime, UNIX_EPOCH};

use nexus_shared::AppError;
use uuid::Uuid;

use crate::models::{CreateLobbyRequest, Lobby, LobbyMember, LobbyStatus};
use crate::stores::lobby_store::LobbyStore;

pub async fn create_lobby(
    lobby_store: &LobbyStore,
    payload: CreateLobbyRequest,
) -> Result<Lobby, AppError> {
    validate_create_lobby_request(&payload)?;

    let lobby = Lobby {
        id: Uuid::new_v4(),
        owner_user_id: payload.owner_user_id,
        name: payload.name,
        status: LobbyStatus::Open,
        members: vec![LobbyMember {
            user_id: payload.owner_user_id,
            joined_at: unix_timestamp(),
        }],
    };

    lobby_store.save_lobby(&lobby).await?;

    Ok(lobby)
}

pub async fn get_lobby(lobby_store: &LobbyStore, id: Uuid) -> Result<Lobby, AppError> {
    lobby_store
        .find_lobby_by_id(id)
        .await?
        .ok_or_else(|| AppError::not_found("lobby not found"))
}

fn validate_create_lobby_request(payload: &CreateLobbyRequest) -> Result<(), AppError> {
    if payload.name.trim().is_empty() {
        return Err(AppError::bad_request("name is required"));
    }

    Ok(())
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is invalid")
        .as_secs()
}
