use async_nats::Client;
use nexus_shared::{
    ensure_stream,
    nats::{EVENTS_FILTER, EVENTS_STREAM},
};

pub async fn ensure_events_stream(nats_client: &Client) {
    ensure_stream(nats_client, EVENTS_STREAM, vec![EVENTS_FILTER.to_string()])
        .await
        .expect("failed to create or get events stream");
}
