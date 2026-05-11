use std::time::Duration;

use async_nats::Client;
use nexus_shared::{AppError, MATCH_FOUND_SUBJECT, MatchFoundEvent, publish_json};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    models::{
        CreateMatchmakingRuleRequest, JoinMatchmakingRequest, MatchmakingRule,
        MatchmakingTicket,
    },
    repositories::matchmaking_rule_repository::MatchmakingRuleRepository,
    stores::matchmaking_store::MatchmakingStore,
};

pub async fn join_matchmaking(
    matchmaking_store: &MatchmakingStore,
    matchmaking_rule_repository: &MatchmakingRuleRepository,
    player_id: Uuid,
    payload: JoinMatchmakingRequest,
) -> Result<MatchmakingTicket, AppError> {
    if payload.ticket_key.trim().is_empty() {
        return Err(AppError::bad_request("ticket_key is required"));
    }

    matchmaking_rule_repository
        .find_enabled_rule_by_ticket_key(&payload.ticket_key)
        .await?
        .ok_or_else(|| AppError::bad_request("ticket_key does not match an enabled rule"))?;

    if matchmaking_store
        .find_ticket_by_player_id(player_id)
        .await?
        .is_some()
    {
        return Err(AppError::conflict("player is already in queue"));
    }

    let ticket = MatchmakingTicket {
        id: Uuid::new_v4(),
        player_id,
        ticket_key: payload.ticket_key.clone(),
    };

    matchmaking_store.save_ticket(&ticket).await?;
    matchmaking_store
        .enqueue_ticket(&ticket.ticket_key, ticket.id)
        .await?;

    Ok(ticket)
}

pub async fn get_matchmaking_status(
    matchmaking_store: &MatchmakingStore,
    player_id: Uuid,
) -> Result<Option<MatchmakingTicket>, AppError> {
    matchmaking_store.find_ticket_by_player_id(player_id).await
}

pub async fn leave_matchmaking(
    matchmaking_store: &MatchmakingStore,
    player_id: Uuid,
) -> Result<(), AppError> {
    let Some(ticket) = matchmaking_store.find_ticket_by_player_id(player_id).await? else {
        return Ok(());
    };

    matchmaking_store
        .remove_ticket_from_queue(&ticket.ticket_key, ticket.id)
        .await?;
    matchmaking_store.delete_ticket(&ticket).await
}

pub async fn create_matchmaking_rule(
    matchmaking_rule_repository: &MatchmakingRuleRepository,
    payload: CreateMatchmakingRuleRequest,
) -> Result<MatchmakingRule, AppError> {
    let ticket_key = payload.ticket_key.trim();
    if ticket_key.is_empty() {
        return Err(AppError::bad_request("ticket_key is required"));
    }

    if payload.required_players < 2 {
        return Err(AppError::bad_request(
            "required_players must be at least 2",
        ));
    }

    matchmaking_rule_repository
        .create_rule(ticket_key, payload.required_players)
        .await
}

pub async fn run_matchmaking_loop(state: AppState) {
    loop {
        if let Err(error) = process_rules(&state).await {
            eprintln!("matchmaking loop error: {}", app_error_status(error));
        }

        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn process_rules(state: &AppState) -> Result<(), AppError> {
    let rules = state.matchmaking_rule_repository.find_enabled_rules().await?;

    for rule in rules {
        matchmake_rule(state, &rule).await?;
    }

    Ok(())
}

async fn matchmake_rule(state: &AppState, rule: &MatchmakingRule) -> Result<(), AppError> {
    let required_players = usize::try_from(rule.required_players)
        .map_err(|_| AppError::internal("stored rule required_players is invalid"))?;

    if required_players < 2 {
        return Err(AppError::internal(
            "stored rule required_players must be at least 2",
        ));
    }

    let queued_tickets = state
        .matchmaking_store
        .peek_queue(&rule.ticket_key, required_players as isize)
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
        .map(|ticket| ticket.player_id)
        .collect::<Vec<_>>();

    publish_match_found_event(
        &state.nats_client,
        MatchFoundEvent {
            rule_id: rule.id,
            ticket_key: rule.ticket_key.clone(),
            player_ids,
        },
    )
    .await?;

    state
        .matchmaking_store
        .remove_queue_prefix(&rule.ticket_key, matched_tickets.len())
        .await?;

    for ticket in matched_tickets {
        state.matchmaking_store.delete_ticket(&ticket).await?;
    }

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
