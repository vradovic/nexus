use async_nats::Client;
use nexus_shared::{
    ensure_stream,
    nats::{EVENTS_FILTER, EVENTS_STREAM},
};

pub const USER_REGISTERED_SUBJECT: &str = "events.auth.user_registered";

pub async fn ensure_events_stream(nats_client: &Client) {
    ensure_stream(nats_client, EVENTS_STREAM, vec![EVENTS_FILTER.to_string()])
        .await
        .expect("failed to create or get events stream");
}
