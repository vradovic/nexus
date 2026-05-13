use std::time::Duration;

use async_nats::Client;
use nexus_shared::{
    AppError, MATCH_CONFIRMED_SUBJECT, MATCH_DECLINED_SUBJECT, MATCH_FOUND_SUBJECT,
    MATCH_TIMED_OUT_SUBJECT, MatchConfirmedEvent, MatchDeclinedEvent, MatchFoundEvent,
    MatchTimedOutEvent, publish_json,
};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    models::{
        CreateMatchmakingRuleRequest, JoinMatchmakingRequest, MatchmakingRule,
        MatchmakingTicket, PendingMatch,
    },
    repository::MatchmakingRuleRepository,
    store::MatchmakingStore,
};

const PENDING_MATCH_TTL_SECONDS: u64 = 30;

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
        .find_pending_match_by_player_id(player_id)
        .await?
        .is_some()
    {
        return Err(AppError::conflict("player is already in a pending match"));
    }

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
) -> Result<(Option<MatchmakingTicket>, Option<PendingMatch>), AppError> {
    let ticket = matchmaking_store.find_ticket_by_player_id(player_id).await?;
    let pending_match = matchmaking_store.find_pending_match_by_player_id(player_id).await?;

    Ok((ticket, pending_match))
}

pub async fn leave_matchmaking(
    matchmaking_store: &MatchmakingStore,
    player_id: Uuid,
) -> Result<(), AppError> {
    if matchmaking_store
        .find_pending_match_by_player_id(player_id)
        .await?
        .is_some()
    {
        return Err(AppError::conflict(
            "player is already in a pending match and cannot leave matchmaking",
        ));
    }

    let Some(ticket) = matchmaking_store.find_ticket_by_player_id(player_id).await? else {
        return Ok(());
    };

    matchmaking_store
        .remove_ticket_from_queue(&ticket.ticket_key, ticket.id)
        .await?;
    matchmaking_store.delete_ticket(&ticket).await
}

pub async fn confirm_match(
    matchmaking_store: &MatchmakingStore,
    nats_client: &Client,
    player_id: Uuid,
    match_id: Uuid,
) -> Result<(), AppError> {
    let Some(mut pending_match) = matchmaking_store.find_pending_match_by_id(match_id).await? else {
        return Err(AppError::not_found("pending match was not found"));
    };

    if is_expired(&pending_match) {
        timeout_pending_match(matchmaking_store, nats_client, &pending_match).await?;
        return Err(AppError::conflict("pending match has already timed out"));
    }

    if !pending_match.player_ids.contains(&player_id) {
        return Err(AppError::forbidden("player is not part of this pending match"));
    }

    if !pending_match.confirmed_player_ids.contains(&player_id) {
        pending_match.confirmed_player_ids.push(player_id);
    }

    if pending_match.confirmed_player_ids.len() == pending_match.player_ids.len() {
        publish_match_confirmed_event(
            nats_client,
            MatchConfirmedEvent {
                match_id: pending_match.id,
                rule_id: pending_match.rule_id,
                ticket_key: pending_match.ticket_key.clone(),
                player_ids: pending_match.player_ids.clone(),
            },
        )
        .await?;

        matchmaking_store.delete_pending_match(&pending_match).await?;
        return Ok(());
    }

    matchmaking_store.save_pending_match(&pending_match).await
}

pub async fn decline_match(
    matchmaking_store: &MatchmakingStore,
    nats_client: &Client,
    player_id: Uuid,
    match_id: Uuid,
) -> Result<(), AppError> {
    let Some(pending_match) = matchmaking_store.find_pending_match_by_id(match_id).await? else {
        return Err(AppError::not_found("pending match was not found"));
    };

    if !pending_match.player_ids.contains(&player_id) {
        return Err(AppError::forbidden("player is not part of this pending match"));
    }

    publish_match_declined_event(
        nats_client,
        MatchDeclinedEvent {
            match_id: pending_match.id,
            rule_id: pending_match.rule_id,
            ticket_key: pending_match.ticket_key.clone(),
            player_ids: pending_match.player_ids.clone(),
            declined_by: player_id,
        },
    )
    .await?;

    matchmaking_store.delete_pending_match(&pending_match).await
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
        if let Err(error) = process_pending_matches(&state).await {
            eprintln!("pending match loop error: {}", app_error_status(error));
        }

        if let Err(error) = process_rules(&state).await {
            eprintln!("matchmaking loop error: {}", app_error_status(error));
        }

        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn process_pending_matches(state: &AppState) -> Result<(), AppError> {
    let pending_matches = state.matchmaking_store.find_all_pending_matches().await?;

    for pending_match in pending_matches {
        if is_expired(&pending_match) {
            timeout_pending_match(&state.matchmaking_store, &state.nats_client, &pending_match)
                .await?;
        }
    }

    Ok(())
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

    let pending_match = PendingMatch {
        id: Uuid::new_v4(),
        rule_id: rule.id,
        ticket_key: rule.ticket_key.clone(),
        player_ids: player_ids.clone(),
        confirmed_player_ids: Vec::new(),
        expires_at_unix_seconds: current_unix_timestamp() + PENDING_MATCH_TTL_SECONDS,
    };

    state.matchmaking_store.save_pending_match(&pending_match).await?;

    publish_match_found_event(
        &state.nats_client,
        MatchFoundEvent {
            match_id: pending_match.id,
            rule_id: rule.id,
            ticket_key: rule.ticket_key.clone(),
            player_ids,
            expires_at_unix_seconds: pending_match.expires_at_unix_seconds,
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

async fn timeout_pending_match(
    matchmaking_store: &MatchmakingStore,
    nats_client: &Client,
    pending_match: &PendingMatch,
) -> Result<(), AppError> {
    publish_match_timed_out_event(
        nats_client,
        MatchTimedOutEvent {
            match_id: pending_match.id,
            rule_id: pending_match.rule_id,
            ticket_key: pending_match.ticket_key.clone(),
            player_ids: pending_match.player_ids.clone(),
        },
    )
    .await?;

    matchmaking_store.delete_pending_match(pending_match).await
}

async fn publish_match_found_event(
    nats_client: &Client,
    event: MatchFoundEvent,
) -> Result<(), AppError> {
    publish_json(nats_client, MATCH_FOUND_SUBJECT, &event).await
}

async fn publish_match_confirmed_event(
    nats_client: &Client,
    event: MatchConfirmedEvent,
) -> Result<(), AppError> {
    publish_json(nats_client, MATCH_CONFIRMED_SUBJECT, &event).await
}

async fn publish_match_declined_event(
    nats_client: &Client,
    event: MatchDeclinedEvent,
) -> Result<(), AppError> {
    publish_json(nats_client, MATCH_DECLINED_SUBJECT, &event).await
}

async fn publish_match_timed_out_event(
    nats_client: &Client,
    event: MatchTimedOutEvent,
) -> Result<(), AppError> {
    publish_json(nats_client, MATCH_TIMED_OUT_SUBJECT, &event).await
}

fn is_expired(pending_match: &PendingMatch) -> bool {
    pending_match.expires_at_unix_seconds <= current_unix_timestamp()
}

fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn app_error_status(error: AppError) -> String {
    let response = axum::response::IntoResponse::into_response(error);
    response.status().to_string()
}
