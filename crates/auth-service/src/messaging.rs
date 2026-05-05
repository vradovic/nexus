use async_nats::Client;
use nexus_shared::ensure_stream;

pub const USER_REGISTERED_STREAM: &str = "AUTH_EVENTS";
pub const USER_REGISTERED_SUBJECT: &str = "auth.user.registered";

pub async fn ensure_registration_stream(nats_client: &Client) {
    ensure_stream(
        nats_client,
        USER_REGISTERED_STREAM,
        vec![USER_REGISTERED_SUBJECT.to_string()],
    )
        .await
        .expect("failed to create or get auth events stream");
}
