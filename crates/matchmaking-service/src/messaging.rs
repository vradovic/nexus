use async_nats::Client;
use nexus_shared::{
    MATCHMAKING_EVENTS_STREAM, MATCH_CONFIRMED_SUBJECT, MATCH_DECLINED_SUBJECT,
    MATCH_FOUND_SUBJECT, MATCH_TIMED_OUT_SUBJECT, ensure_stream,
};

pub async fn ensure_matchmaking_stream(nats_client: &Client) {
    ensure_stream(
        nats_client,
        MATCHMAKING_EVENTS_STREAM,
        vec![
            MATCH_FOUND_SUBJECT.to_string(),
            MATCH_CONFIRMED_SUBJECT.to_string(),
            MATCH_DECLINED_SUBJECT.to_string(),
            MATCH_TIMED_OUT_SUBJECT.to_string(),
        ],
    )
    .await
    .expect("failed to create or get matchmaking events stream");
}
