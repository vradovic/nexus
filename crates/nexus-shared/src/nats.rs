use async_nats::{
    jetstream,
    jetstream::{
        consumer::pull::{self, Stream},
        message::Acker,
    },
};
use bytes::Bytes;
use futures_util::StreamExt;
use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

pub const EVENTS_STREAM: &str = "EVENTS";
pub const EVENTS_FILTER: &str = "events.>";

pub const COMMANDS_STREAM: &str = "COMMANDS";
pub const COMMANDS_FILTER: &str = "commands.>";

#[derive(Debug, Error)]
pub enum NatsError {
    #[error("failed to connect to nats: {reason}")]
    ConnectionFailed { reason: String },
    #[error("stream {stream_name} does not exist: {reason}")]
    StreamNotFound { stream_name: String, reason: String },
    #[error(
        "failed to create or update consumer {consumer_name} for stream {stream_name}: {reason}"
    )]
    ConsumerUpdateFail {
        stream_name: String,
        consumer_name: String,
        reason: String,
    },
    #[error(
        "failed to fetch messages for consumer {consumer_name} on stream {stream_name}: {reason}"
    )]
    MessageStreamFail {
        stream_name: String,
        consumer_name: String,
        reason: String,
    },
    #[error("failed to read nats message: {reason}")]
    MessageReadFailed { reason: String },
    #[error("failed to ack nats message: {reason}")]
    AckFailed { reason: String },
    #[error("failed to publish nats message on subject {subject}: {reason}")]
    PublishFailed { subject: String, reason: String },
    #[error("failed to serialize nats payload on subject {subject}: {source}")]
    SerializeFailed {
        subject: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to deserialize nats payload on subject {subject}: {source}")]
    DeserializeFailed {
        subject: String,
        #[source]
        source: serde_json::Error,
    },
}

pub struct NatsAdapter {
    jetstream_ctx: jetstream::Context,
}

pub struct MessageReader {
    inner: Stream,
}

pub struct NatsMessage {
    subject: String,
    payload: Bytes,
    acker: Acker,
}

pub struct NatsCommandMessage {
    pub subject: String,
    pub payload: Bytes,
}

impl NatsAdapter {
    pub async fn new(url: &str) -> Result<Self, NatsError> {
        let client =
            async_nats::connect(url)
                .await
                .map_err(|error| NatsError::ConnectionFailed {
                    reason: error.to_string(),
                })?;
        let jetstream_ctx = jetstream::new(client);

        ensure_stream_exists(&jetstream_ctx, EVENTS_STREAM).await?;
        ensure_stream_exists(&jetstream_ctx, COMMANDS_STREAM).await?;

        Ok(Self { jetstream_ctx })
    }

    pub async fn events_reader(&self, consumer_name: &str) -> Result<MessageReader, NatsError> {
        self.ensure_pull_consumer(EVENTS_STREAM, consumer_name, EVENTS_FILTER)
            .await
    }

    pub async fn commands_reader(&self, consumer_name: &str) -> Result<MessageReader, NatsError> {
        self.ensure_pull_consumer(COMMANDS_STREAM, consumer_name, COMMANDS_FILTER)
            .await
    }

    pub async fn ensure_pull_consumer(
        &self,
        stream_name: &str,
        consumer_name: &str,
        filter_subject: &str,
    ) -> Result<MessageReader, NatsError> {
        let config = pull::Config {
            durable_name: Some(consumer_name.to_string()),
            filter_subject: filter_subject.to_string(),
            ..Default::default()
        };

        let consumer: jetstream::consumer::Consumer<pull::Config> = self
            .jetstream_ctx
            .create_consumer_on_stream(config, stream_name)
            .await
            .map_err(|error| NatsError::ConsumerUpdateFail {
                stream_name: stream_name.to_string(),
                consumer_name: consumer_name.to_string(),
                reason: error.to_string(),
            })?;

        let inner = consumer
            .messages()
            .await
            .map_err(|error| NatsError::MessageStreamFail {
                stream_name: stream_name.to_string(),
                consumer_name: consumer_name.to_string(),
                reason: error.to_string(),
            })?;

        Ok(MessageReader { inner })
    }

    pub async fn publish(&self, subject: String, payload: Bytes) -> Result<(), NatsError> {
        self.publish_bytes(subject, payload).await
    }

    pub async fn publish_json<T>(&self, subject: &str, payload: &T) -> Result<(), NatsError>
    where
        T: Serialize,
    {
        let payload = serde_json::to_vec(payload).map_err(|error| NatsError::SerializeFailed {
            subject: subject.to_string(),
            source: error,
        })?;

        self.publish_bytes(subject.to_string(), payload.into())
            .await
    }

    pub async fn write_commands(&self, commands: Vec<NatsCommandMessage>) -> Result<(), NatsError> {
        for NatsCommandMessage { subject, payload } in commands {
            self.publish_bytes(subject, payload).await?;
        }

        Ok(())
    }

    async fn publish_bytes(&self, subject: String, payload: Bytes) -> Result<(), NatsError> {
        let ack = self
            .jetstream_ctx
            .publish(subject.clone(), payload)
            .await
            .map_err(|error| NatsError::PublishFailed {
                subject: subject.clone(),
                reason: error.to_string(),
            })?;

        ack.await.map_err(|error| NatsError::PublishFailed {
            subject,
            reason: error.to_string(),
        })?;

        Ok(())
    }
}

impl MessageReader {
    pub async fn next(&mut self) -> Result<Option<NatsMessage>, NatsError> {
        let Some(message) = self.inner.next().await else {
            return Ok(None);
        };

        let (message, acker) = message
            .map_err(|error| NatsError::MessageReadFailed {
                reason: error.to_string(),
            })?
            .split();

        Ok(Some(NatsMessage {
            subject: message.subject.into_string(),
            payload: message.payload,
            acker,
        }))
    }
}

impl NatsMessage {
    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn payload_bytes(&self) -> &Bytes {
        &self.payload
    }

    pub fn decode<T>(&self) -> Result<T, NatsError>
    where
        T: DeserializeOwned,
    {
        serde_json::from_slice(&self.payload).map_err(|error| NatsError::DeserializeFailed {
            subject: self.subject.clone(),
            source: error,
        })
    }

    pub async fn ack(self) -> Result<(), NatsError> {
        self.acker
            .ack()
            .await
            .map_err(|error| NatsError::AckFailed {
                reason: error.to_string(),
            })
    }
}

async fn ensure_stream_exists(
    jetstream_ctx: &jetstream::Context,
    stream_name: &str,
) -> Result<(), NatsError> {
    jetstream_ctx
        .get_stream(stream_name)
        .await
        .map_err(|error| NatsError::StreamNotFound {
            stream_name: stream_name.to_string(),
            reason: error.to_string(),
        })?;

    Ok(())
}
