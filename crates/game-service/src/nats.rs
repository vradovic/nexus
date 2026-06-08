use async_nats::{
    Client, jetstream,
    jetstream::{consumer::pull, message::Acker},
};
use futures_util::StreamExt;

const EVENTS_STREAM: &str = "EVENTS";
const EVENTS_FILTER: &str = "events.>";
const EVENTS_CONSUMER: &str = "game-service";
const COMMANDS_STREAM: &str = "COMMANDS";
const COMMANDS_FILTER: &str = "commands.>";

pub struct NatsEventMessage {
    pub subject: String,
    pub payload: Vec<u8>,
    pub acker: Acker,
}

pub struct NatsEventReader {
    messages: pull::Stream,
}

pub struct NatsCommandWriter {
    jetstream: jetstream::Context,
}

impl NatsEventReader {
    pub async fn new(nats_client: Client) -> Result<Self, Box<dyn std::error::Error>> {
        tracing::info!(
            stream = EVENTS_STREAM,
            consumer = EVENTS_CONSUMER,
            filter = EVENTS_FILTER,
            "starting nats consumer"
        );

        let jetstream = jetstream::new(nats_client);

        let stream = jetstream.get_stream(EVENTS_STREAM).await?;

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

        Ok(Self { messages })
    }

    pub async fn read_message(
        &mut self,
    ) -> Result<Option<NatsEventMessage>, Box<dyn std::error::Error>> {
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
}

impl NatsCommandWriter {
    pub async fn new(nats_client: Client) -> Result<Self, Box<dyn std::error::Error>> {
        let jetstream = jetstream::new(nats_client);

        jetstream
            .create_or_update_stream(jetstream::stream::Config {
                name: COMMANDS_STREAM.to_string(),
                subjects: vec![COMMANDS_FILTER.to_string()],
                ..Default::default()
            })
            .await?;

        tracing::info!(
            stream = COMMANDS_STREAM,
            filter = COMMANDS_FILTER,
            "nats command writer ready"
        );

        Ok(Self { jetstream })
    }
}
