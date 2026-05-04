use async_nats::{Client, jetstream};

pub const USER_REGISTERED_STREAM: &str = "AUTH_EVENTS";
pub const USER_REGISTERED_SUBJECT: &str = "auth.user.registered";

pub async fn ensure_registration_stream(nats_client: &Client) {
    let jetstream = jetstream::new(nats_client.clone());

    jetstream
        .get_or_create_stream(jetstream::stream::Config {
            name: USER_REGISTERED_STREAM.to_string(),
            subjects: vec![USER_REGISTERED_SUBJECT.to_string()],
            ..Default::default()
        })
        .await
        .expect("failed to create or get auth events stream");
}
