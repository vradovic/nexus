use async_nats::Client;
use nexus_shared::{MATCH_FOUND_SUBJECT, REALTIME_EVENTS_STREAM, ensure_stream};

pub async fn ensure_realtime_stream(nats_client: &Client) {
    ensure_stream(
        nats_client,
        REALTIME_EVENTS_STREAM,
        vec![MATCH_FOUND_SUBJECT.to_string()],
    )
        .await
        .expect("failed to create or get realtime events stream");
}
