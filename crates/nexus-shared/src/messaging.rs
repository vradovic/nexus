use async_nats::{Client, jetstream};
use serde::Serialize;

use crate::AppError;

pub async fn ensure_stream(
    nats_client: &Client,
    stream_name: &str,
    subjects: Vec<String>,
) -> Result<(), AppError> {
    jetstream::new(nats_client.clone())
        .get_or_create_stream(jetstream::stream::Config {
            name: stream_name.to_string(),
            subjects,
            ..Default::default()
        })
        .await
        .map_err(|_| AppError::internal("failed to create or get nats stream"))?;

    Ok(())
}

pub async fn ensure_pull_consumer(
    nats_client: &Client,
    stream_name: &str,
    consumer_name: &str,
    filter_subject: &str,
) -> Result<(), AppError> {
    let jetstream = jetstream::new(nats_client.clone());
    let stream = jetstream
        .get_stream(stream_name)
        .await
        .map_err(|_| AppError::internal("failed to get nats stream"))?;

    stream
        .get_or_create_consumer(
            consumer_name,
            jetstream::consumer::pull::Config {
                durable_name: Some(consumer_name.to_string()),
                filter_subject: filter_subject.to_string(),
                ..Default::default()
            },
        )
        .await
        .map_err(|_| AppError::internal("failed to create or get nats consumer"))?;

    Ok(())
}

pub async fn publish_json<T>(
    nats_client: &Client,
    subject: &str,
    payload: &T,
) -> Result<(), AppError>
where
    T: Serialize,
{
    let body = serde_json::to_vec(payload)
        .map_err(|_| AppError::internal("failed to serialize nats payload"))?;

    jetstream::new(nats_client.clone())
        .publish(subject.to_string(), body.into())
        .await
        .map_err(|_| AppError::internal("failed to publish nats message"))?;

    Ok(())
}
