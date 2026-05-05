use std::time::Duration;

use async_nats::Client;
use nexus_shared::{AppError, GameMode, MATCH_FOUND_SUBJECT, MatchFoundEvent, publish_json};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    models::{JoinMatchmakingRequest, MatchmakingTicket, MatchmakingTicketStatus},
    stores::matchmaking_store::MatchmakingStore,
};

use super::game_session_service;

pub async fn join_matchmaking(
    matchmaking_store: &MatchmakingStore,
    user_id: Uuid,
    payload: JoinMatchmakingRequest,
) -> Result<MatchmakingTicket, AppError> {
    if let Some(ticket) = matchmaking_store.find_ticket_by_user_id(user_id).await? {
        return match ticket.status {
            MatchmakingTicketStatus::Queued => Err(AppError::conflict("user is already in queue")),
            MatchmakingTicketStatus::Matched => {
                Err(AppError::conflict("user is already assigned to a match"))
            }
        };
    }

    let ticket = MatchmakingTicket {
        id: Uuid::new_v4(),
        owner_user_id: user_id,
        player_ids: vec![user_id],
        game_mode: payload.game_mode.clone(),
        status: MatchmakingTicketStatus::Queued,
        game_session_id: None,
    };

    matchmaking_store.save_ticket(&ticket).await?;
    matchmaking_store
        .enqueue_ticket(&payload.game_mode, ticket.id)
        .await?;

    Ok(ticket)
}

pub async fn run_matchmaking_loop(state: AppState) {
    loop {
        if let Err(error) = matchmake_game_mode(&state, GameMode::Duel1v1).await {
            eprintln!("matchmaking loop error: {}", app_error_status(error));
        }

        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn matchmake_game_mode(state: &AppState, game_mode: GameMode) -> Result<(), AppError> {
    let required_players = game_mode.required_players();
    let queued_tickets = state
        .matchmaking_store
        .peek_queue(&game_mode, required_players as isize)
        .await?;

    if queued_tickets.len() < required_players {
        return Ok(());
    }

    let matched_tickets = queued_tickets
        .into_iter()
        .take(required_players)
        .collect::<Vec<_>>();

    let player_ids = matched_tickets
        .iter()
        .flat_map(|ticket| ticket.player_ids.iter().copied())
        .collect::<Vec<_>>();

    if player_ids.len() != required_players {
        return Ok(());
    }

    let game_session =
        game_session_service::create_game_session(&state.game_session_store, player_ids.clone())
            .await?;

    for ticket in matched_tickets.iter() {
        let updated_ticket = MatchmakingTicket {
            id: ticket.id,
            owner_user_id: ticket.owner_user_id,
            player_ids: ticket.player_ids.clone(),
            game_mode: ticket.game_mode.clone(),
            status: MatchmakingTicketStatus::Matched,
            game_session_id: Some(game_session.id),
        };

        state.matchmaking_store.save_ticket(&updated_ticket).await?;
    }

    state
        .matchmaking_store
        .remove_queue_prefix(&game_mode, matched_tickets.len())
        .await?;

    publish_match_found_event(
        &state.nats_client,
        MatchFoundEvent {
            game_session_id: game_session.id,
            game_mode,
            user_ids: player_ids,
        },
    )
    .await?;

    Ok(())
}

async fn publish_match_found_event(
    nats_client: &Client,
    event: MatchFoundEvent,
) -> Result<(), AppError> {
    publish_json(nats_client, MATCH_FOUND_SUBJECT, &event).await
}

fn app_error_status(error: AppError) -> String {
    let response = axum::response::IntoResponse::into_response(error);
    response.status().to_string()
}
