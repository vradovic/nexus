use std::error::Error;

use async_nats::{
    Client, jetstream,
    jetstream::{consumer::pull, message::Acker},
};
use bytes::Bytes;
use futures_util::StreamExt;

const EVENTS_STREAM: &str = "EVENTS";
const EVENTS_FILTER: &str = "events.>";
const EVENTS_CONSUMER: &str = "game-service";
const COMMANDS_STREAM: &str = "COMMANDS";

pub struct NatsEventMessage {
    pub subject: String,
    pub payload: Vec<u8>,
    pub acker: Acker,
}

pub struct NatsCommandMessage {
    pub subject: String,
    pub payload: Bytes,
}

pub struct NatsAdapter {
    ctx: jetstream::Context,
    messages: pull::Stream,
}

impl NatsAdapter {
    pub async fn new(nats_client: Client) -> Result<Self, Box<dyn std::error::Error>> {
        tracing::info!(
            stream = EVENTS_STREAM,
            consumer = EVENTS_CONSUMER,
            filter = EVENTS_FILTER,
            "starting nats consumer"
        );

        let jetstream = jetstream::new(nats_client.clone());

        let stream = jetstream.get_stream(EVENTS_STREAM).await?;

        jetstream.get_stream(COMMANDS_STREAM).await?; // ensure commands stream exists

        let consumer: jetstream::consumer::Consumer<jetstream::consumer::pull::Config> = stream
            .get_or_create_consumer::<jetstream::consumer::pull::Config>(
                EVENTS_CONSUMER,
                jetstream::consumer::pull::Config {
                    durable_name: Some(EVENTS_CONSUMER.to_string()),
                    filter_subject: EVENTS_FILTER.to_string(),
                    ..Default::default()
                },
            )
            .await?;

        tracing::info!(
            stream = EVENTS_STREAM,
            consumer = EVENTS_CONSUMER,
            filter = EVENTS_FILTER,
            "nats consumer ready"
        );

        let messages = consumer.messages().await?;

        Ok(Self {
            ctx: jetstream,
            messages,
        })
    }

    pub async fn read_events(&mut self) -> Result<Option<NatsEventMessage>, Box<dyn Error>> {
        let Some(message) = self.messages.next().await else {
            return Ok(None);
        };

        let (message, acker) = message?.split();

        tracing::debug!(
            subject = %message.subject,
            payload_size = message.payload.len(),
            "received game event"
        );

        Ok(Some(NatsEventMessage {
            subject: message.subject.into_string(),
            payload: message.payload.to_vec(),
            acker,
        }))
    }

    pub async fn write_commands(
        &self,
        commands: Vec<NatsCommandMessage>,
    ) -> Result<(), Box<dyn Error>> {
        for NatsCommandMessage { subject, payload } in commands {
            let ack = self.ctx.publish(subject, payload).await?;
            ack.await?;
        }

        Ok(())
    }
}

pub async fn ack_message(acker: Acker) -> Result<(), Box<dyn Error>> {
    acker
        .ack()
        .await
        .map_err(|error| -> Box<dyn Error> { error })?;
    tracing::debug!("acked game event");

    Ok(())
}
