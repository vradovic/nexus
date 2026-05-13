#![allow(dead_code)]

use std::time::Duration;

use async_nats::jetstream;
use futures_util::StreamExt;
use nexus_shared::MATCHMAKING_EVENTS_STREAM;
use redis::AsyncCommands;
use reqwest::StatusCode;
use serial_test::serial;
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

const AUTH_URL: &str = "http://127.0.0.1:3001";
const MATCHMAKING_URL: &str = "http://127.0.0.1:3003";
const REALTIME_HTTP_URL: &str = "http://127.0.0.1:3004";
const REALTIME_URL: &str = "ws://127.0.0.1:3004/ws";
const NATS_URL: &str = "nats://127.0.0.1:4222";
const REDIS_URL: &str = "redis://127.0.0.1:6379";
const TICKET_KEY: &str = "duel";

#[derive(Debug, Deserialize)]
struct RegisterResponse {
    id: Uuid,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct MatchmakingTicket {
    id: Uuid,
    player_id: Uuid,
    ticket_key: String,
}

#[derive(Debug, Deserialize, Clone)]
struct PendingMatch {
    id: Uuid,
    rule_id: Uuid,
    ticket_key: String,
    player_ids: Vec<Uuid>,
    confirmed_player_ids: Vec<Uuid>,
    expires_at_unix_seconds: u64,
}

#[derive(Debug, Deserialize)]
struct MatchmakingStatusResponse {
    ticket: Option<MatchmakingTicket>,
    pending_match: Option<PendingMatch>,
}

#[derive(Debug, Serialize)]
struct RegisterRequest {
    email: String,
    username: String,
    first_name: String,
    last_name: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct JoinRequest {
    ticket_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RealtimeEvent {
    Connected {
        user_id: Uuid,
        email: String,
    },
    MatchFound {
        match_id: Uuid,
        rule_id: Uuid,
        ticket_key: String,
        player_ids: Vec<Uuid>,
        expires_at_unix_seconds: u64,
    },
    MatchConfirmed {
        match_id: Uuid,
        rule_id: Uuid,
        ticket_key: String,
        player_ids: Vec<Uuid>,
    },
    MatchDeclined {
        match_id: Uuid,
        rule_id: Uuid,
        ticket_key: String,
        player_ids: Vec<Uuid>,
        declined_by: Uuid,
    },
    MatchTimedOut {
        match_id: Uuid,
        rule_id: Uuid,
        ticket_key: String,
        player_ids: Vec<Uuid>,
    },
}

#[derive(Debug, Clone, Copy)]
struct FoundMatch {
    match_id: Uuid,
    expires_at_unix_seconds: u64,
}

type WsStream = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

#[tokio::test]
#[ignore = "requires local docker infra and running auth/matchmaking/realtime services"]
#[serial]
async fn two_players_can_match_and_confirm() {
    ensure_services_are_healthy().await;
    reset_matchmaking_redis_state().await;
    reset_matchmaking_nats_state().await;

    let client = reqwest::Client::new();
    let (player_one_id, player_one_token) = register_and_login(&client, "confirm-p1").await;
    let (player_two_id, player_two_token) = register_and_login(&client, "confirm-p2").await;

    let mut player_one_socket = connect_ws(&player_one_token).await;
    let mut player_two_socket = connect_ws(&player_two_token).await;

    assert_connected_event(&mut player_one_socket, player_one_id).await;
    assert_connected_event(&mut player_two_socket, player_two_id).await;

    join_queue(&client, &player_one_token).await;
    join_queue(&client, &player_two_token).await;

    let player_one_match = assert_match_found(
        &mut player_one_socket,
        &[player_one_id, player_two_id],
    )
    .await;
    let player_two_match = assert_match_found(
        &mut player_two_socket,
        &[player_one_id, player_two_id],
    )
    .await;
    assert_eq!(player_one_match.match_id, player_two_match.match_id);

    let pending_status = get_status(&client, &player_one_token).await;
    let pending_match = pending_status
        .pending_match
        .expect("player one should have a pending match after match_found");
    assert_eq!(pending_match.id, player_one_match.match_id);
    assert_eq!(pending_match.confirmed_player_ids.len(), 0);

    confirm_match(&client, &player_one_token, player_one_match.match_id).await;
    let pending_status = get_status(&client, &player_one_token).await;
    let pending_match = pending_status
        .pending_match
        .expect("pending match should still exist after first confirmation");
    assert_eq!(pending_match.confirmed_player_ids, vec![player_one_id]);

    confirm_match(&client, &player_two_token, player_one_match.match_id).await;

    assert_match_confirmed(
        &mut player_one_socket,
        player_one_match.match_id,
        &[player_one_id, player_two_id],
    )
    .await;
    assert_match_confirmed(
        &mut player_two_socket,
        player_one_match.match_id,
        &[player_one_id, player_two_id],
    )
    .await;

    let final_status = get_status(&client, &player_one_token).await;
    assert!(
        final_status.ticket.is_none(),
        "queue ticket should be cleared after confirm flow"
    );
    assert!(
        final_status.pending_match.is_none(),
        "pending match should be deleted after all players confirm"
    );
}

#[tokio::test]
#[ignore = "requires local docker infra and running auth/matchmaking/realtime services"]
#[serial]
async fn pending_match_blocks_join_and_leave_and_decline_notifies_players() {
    ensure_services_are_healthy().await;
    reset_matchmaking_redis_state().await;
    reset_matchmaking_nats_state().await;

    let client = reqwest::Client::new();
    let (player_one_id, player_one_token) = register_and_login(&client, "decline-p1").await;
    let (player_two_id, player_two_token) = register_and_login(&client, "decline-p2").await;

    let mut player_one_socket = connect_ws(&player_one_token).await;
    let mut player_two_socket = connect_ws(&player_two_token).await;

    assert_connected_event(&mut player_one_socket, player_one_id).await;
    assert_connected_event(&mut player_two_socket, player_two_id).await;

    join_queue(&client, &player_one_token).await;
    join_queue(&client, &player_two_token).await;

    let player_one_match = assert_match_found(
        &mut player_one_socket,
        &[player_one_id, player_two_id],
    )
    .await;
    let _ = assert_match_found(&mut player_two_socket, &[player_one_id, player_two_id]).await;

    let join_response = client
        .post(format!("{MATCHMAKING_URL}/join"))
        .bearer_auth(&player_one_token)
        .json(&JoinRequest {
            ticket_key: TICKET_KEY.to_string(),
        })
        .send()
        .await
        .expect("failed to call join while pending");
    assert_eq!(join_response.status(), StatusCode::CONFLICT);

    let leave_response = client
        .post(format!("{MATCHMAKING_URL}/leave"))
        .bearer_auth(&player_one_token)
        .send()
        .await
        .expect("failed to call leave while pending");
    assert_eq!(leave_response.status(), StatusCode::CONFLICT);

    decline_match(&client, &player_one_token, player_one_match.match_id).await;

    assert_match_declined(
        &mut player_one_socket,
        player_one_match.match_id,
        player_one_id,
        &[player_one_id, player_two_id],
    )
    .await;
    assert_match_declined(
        &mut player_two_socket,
        player_one_match.match_id,
        player_one_id,
        &[player_one_id, player_two_id],
    )
    .await;

    let final_status = get_status(&client, &player_one_token).await;
    assert!(
        final_status.ticket.is_none(),
        "queue ticket should be cleared after decline flow"
    );
    assert!(
        final_status.pending_match.is_none(),
        "pending match should be deleted after a decline"
    );
}

#[tokio::test]
#[ignore = "requires local docker infra and running auth/matchmaking/realtime services"]
#[serial]
async fn pending_match_times_out_and_notifies_players() {
    ensure_services_are_healthy().await;
    reset_matchmaking_redis_state().await;
    reset_matchmaking_nats_state().await;

    let client = reqwest::Client::new();
    let (player_one_id, player_one_token) = register_and_login(&client, "timeout-p1").await;
    let (player_two_id, player_two_token) = register_and_login(&client, "timeout-p2").await;

    let mut player_one_socket = connect_ws(&player_one_token).await;
    let mut player_two_socket = connect_ws(&player_two_token).await;

    assert_connected_event(&mut player_one_socket, player_one_id).await;
    assert_connected_event(&mut player_two_socket, player_two_id).await;

    join_queue(&client, &player_one_token).await;
    join_queue(&client, &player_two_token).await;

    let player_one_match = assert_match_found(
        &mut player_one_socket,
        &[player_one_id, player_two_id],
    )
    .await;
    let _ = assert_match_found(&mut player_two_socket, &[player_one_id, player_two_id]).await;

    let pending_status = get_status(&client, &player_one_token).await;
    assert!(
        pending_status.pending_match.is_some(),
        "pending match should exist immediately after match_found"
    );

    let wait_duration = timeout_wait_duration(player_one_match.expires_at_unix_seconds);

    assert_match_timed_out(
        &mut player_one_socket,
        player_one_match.match_id,
        &[player_one_id, player_two_id],
        wait_duration,
    )
    .await;
    assert_match_timed_out(
        &mut player_two_socket,
        player_one_match.match_id,
        &[player_one_id, player_two_id],
        Duration::from_secs(5),
    )
    .await;

    let final_status = get_status(&client, &player_one_token).await;
    assert!(
        final_status.ticket.is_none(),
        "queue ticket should be cleared after timeout flow"
    );
    assert!(
        final_status.pending_match.is_none(),
        "pending match should be deleted after timeout"
    );
}

async fn ensure_services_are_healthy() {
    let client = reqwest::Client::new();
    let required_services = [
        ("auth-service", format!("{AUTH_URL}/health")),
        ("matchmaking-service", format!("{MATCHMAKING_URL}/health")),
        ("realtime-service", format!("{REALTIME_HTTP_URL}/health")),
    ];

    let mut unavailable_services = Vec::new();

    for (service_name, url) in required_services {
        match client.get(&url).send().await {
            Ok(response) if response.status() == StatusCode::OK => {}
            Ok(response) => unavailable_services.push(format!(
                "- {service_name} responded with unexpected status {} at {url}",
                response.status()
            )),
            Err(error) => unavailable_services.push(format!(
                "- {service_name} is not reachable at {url}: {error}"
            )),
        }
    }

    if !unavailable_services.is_empty() {
        panic!(
            "integration smoke tests require the local stack to already be running.\n\
             Start the infrastructure and services first, then rerun the test.\n\n\
             Required steps:\n\
             1. docker compose up -d\n\
             2. cargo run -p auth-service\n\
             3. cargo run -p matchmaking-service\n\
             4. cargo run -p realtime-service\n\n\
             Unavailable services:\n{}",
            unavailable_services.join("\n")
        );
    }
}

async fn reset_matchmaking_redis_state() {
    let redis_client = redis::Client::open(REDIS_URL).expect("failed to create redis client for tests");
    let mut connection = redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("failed to connect to redis for tests");

    let mut cursor = 0u64;
    let mut keys_to_delete = Vec::new();

    loop {
        let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg("matchmaking:*")
            .query_async(&mut connection)
            .await
            .expect("failed to scan redis matchmaking keys");

        keys_to_delete.extend(keys);
        if next_cursor == 0 {
            break;
        }

        cursor = next_cursor;
    }

    if !keys_to_delete.is_empty() {
        connection
            .del::<_, ()>(keys_to_delete)
            .await
            .expect("failed to delete existing matchmaking redis keys before test");
    }
}

async fn reset_matchmaking_nats_state() {
    let nats_client = async_nats::connect(NATS_URL)
        .await
        .expect("failed to connect to nats for tests");
    let jetstream = jetstream::new(nats_client);
    let stream = jetstream
        .get_stream(MATCHMAKING_EVENTS_STREAM)
        .await
        .expect("failed to get matchmaking events stream for tests");

    stream
        .purge()
        .await
        .expect("failed to purge matchmaking events stream before test");
}

async fn register_and_login(client: &reqwest::Client, label: &str) -> (Uuid, String) {
    let suffix = Uuid::new_v4().simple().to_string();
    let email = format!("{label}-{suffix}@example.com");
    let username = format!("{}_{}", label.replace('-', "_"), &suffix[..8]);
    let password = "lozinka123".to_string();

    let register_response = client
        .post(format!("{AUTH_URL}/register"))
        .json(&RegisterRequest {
            email: email.clone(),
            username,
            first_name: "Test".to_string(),
            last_name: "Player".to_string(),
            password: password.clone(),
        })
        .send()
        .await
        .expect("failed to call register");
    assert_eq!(register_response.status(), StatusCode::CREATED);
    let register_body = register_response
        .json::<RegisterResponse>()
        .await
        .expect("failed to decode register response");

    let login_response = client
        .post(format!("{AUTH_URL}/login"))
        .json(&LoginRequest { email, password })
        .send()
        .await
        .expect("failed to call login");
    assert_eq!(login_response.status(), StatusCode::OK);
    let login_body = login_response
        .json::<LoginResponse>()
        .await
        .expect("failed to decode login response");

    (register_body.id, login_body.access_token)
}

async fn connect_ws(token: &str) -> WsStream {
    let (socket, _response) = connect_async(format!("{REALTIME_URL}?token={token}"))
        .await
        .expect("failed to connect websocket");
    socket
}

async fn join_queue(client: &reqwest::Client, token: &str) {
    let response = client
        .post(format!("{MATCHMAKING_URL}/join"))
        .bearer_auth(token)
        .json(&JoinRequest {
            ticket_key: TICKET_KEY.to_string(),
        })
        .send()
        .await
        .expect("failed to join matchmaking");
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let ticket = response
        .json::<MatchmakingTicket>()
        .await
        .expect("failed to decode join response");
    assert_eq!(ticket.ticket_key, TICKET_KEY);
}

async fn confirm_match(client: &reqwest::Client, token: &str, match_id: Uuid) {
    let response = client
        .post(format!("{MATCHMAKING_URL}/matches/{match_id}/confirm"))
        .bearer_auth(token)
        .send()
        .await
        .expect("failed to confirm match");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

async fn decline_match(client: &reqwest::Client, token: &str, match_id: Uuid) {
    let response = client
        .post(format!("{MATCHMAKING_URL}/matches/{match_id}/decline"))
        .bearer_auth(token)
        .send()
        .await
        .expect("failed to decline match");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

async fn get_status(client: &reqwest::Client, token: &str) -> MatchmakingStatusResponse {
    let response = client
        .get(format!("{MATCHMAKING_URL}/status"))
        .bearer_auth(token)
        .send()
        .await
        .expect("failed to get matchmaking status");
    assert_eq!(response.status(), StatusCode::OK);
    response
        .json::<MatchmakingStatusResponse>()
        .await
        .expect("failed to decode matchmaking status")
}

async fn assert_connected_event(socket: &mut WsStream, expected_user_id: Uuid) {
    match next_event(socket, Duration::from_secs(10), "connected event").await {
        RealtimeEvent::Connected { user_id, .. } => assert_eq!(user_id, expected_user_id),
        other => panic!("expected connected event, got {other:?}"),
    }
}

async fn assert_match_found(socket: &mut WsStream, expected_players: &[Uuid]) -> FoundMatch {
    match next_event(socket, Duration::from_secs(10), "match_found event").await {
        RealtimeEvent::MatchFound {
            match_id,
            ticket_key,
            player_ids,
            expires_at_unix_seconds,
            ..
        } => {
            assert_eq!(ticket_key, TICKET_KEY);
            assert_same_players(&player_ids, expected_players);
            FoundMatch {
                match_id,
                expires_at_unix_seconds,
            }
        }
        other => panic!("expected match_found event, got {other:?}"),
    }
}

async fn assert_match_confirmed(
    socket: &mut WsStream,
    expected_match_id: Uuid,
    expected_players: &[Uuid],
) {
    match next_event(socket, Duration::from_secs(10), "match_confirmed event").await {
        RealtimeEvent::MatchConfirmed {
            match_id,
            ticket_key,
            player_ids,
            ..
        } => {
            assert_eq!(match_id, expected_match_id);
            assert_eq!(ticket_key, TICKET_KEY);
            assert_same_players(&player_ids, expected_players);
        }
        other => panic!("expected match_confirmed event, got {other:?}"),
    }
}

async fn assert_match_declined(
    socket: &mut WsStream,
    expected_match_id: Uuid,
    expected_declined_by: Uuid,
    expected_players: &[Uuid],
) {
    match next_event(socket, Duration::from_secs(10), "match_declined event").await {
        RealtimeEvent::MatchDeclined {
            match_id,
            ticket_key,
            player_ids,
            declined_by,
            ..
        } => {
            assert_eq!(match_id, expected_match_id);
            assert_eq!(ticket_key, TICKET_KEY);
            assert_eq!(declined_by, expected_declined_by);
            assert_same_players(&player_ids, expected_players);
        }
        other => panic!("expected match_declined event, got {other:?}"),
    }
}

async fn assert_match_timed_out(
    socket: &mut WsStream,
    expected_match_id: Uuid,
    expected_players: &[Uuid],
    wait_duration: Duration,
) {
    match next_event(socket, wait_duration, "match_timed_out event").await {
        RealtimeEvent::MatchTimedOut {
            match_id,
            ticket_key,
            player_ids,
            ..
        } => {
            assert_eq!(match_id, expected_match_id);
            assert_eq!(ticket_key, TICKET_KEY);
            assert_same_players(&player_ids, expected_players);
        }
        other => panic!("expected match_timed_out event, got {other:?}"),
    }
}

async fn next_event(socket: &mut WsStream, wait_duration: Duration, context: &str) -> RealtimeEvent {
    loop {
        let message = timeout(wait_duration, socket.next())
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for {context}"))
            .unwrap_or_else(|| panic!("websocket closed before receiving {context}"))
            .unwrap_or_else(|error| panic!("failed to read websocket message while waiting for {context}: {error}"));

        match message {
            Message::Text(text) => {
                return serde_json::from_str::<RealtimeEvent>(&text)
                    .expect("failed to decode realtime event");
            }
            Message::Binary(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
            Message::Close(frame) => panic!("websocket closed unexpectedly: {frame:?}"),
        }
    }
}

fn timeout_wait_duration(expires_at_unix_seconds: u64) -> Duration {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let remaining_seconds = expires_at_unix_seconds.saturating_sub(now);
    Duration::from_secs(remaining_seconds + 5)
}

fn assert_same_players(actual: &[Uuid], expected: &[Uuid]) {
    let mut actual_players = actual.to_vec();
    let mut expected_players = expected.to_vec();
    actual_players.sort();
    expected_players.sort();
    assert_eq!(actual_players, expected_players);
}
