use std::error::Error;

use async_nats::{Client, jetstream::message::Acker};

use crate::{
    engine::ScriptEngine,
    nats::{NatsCommandWriter, NatsEventReader},
};

pub struct Game {
    reader: NatsEventReader,
    writer: NatsCommandWriter,
    engine: ScriptEngine,
}

impl Game {
    pub async fn new(nats_client: Client, script_path: &str) -> Result<Self, Box<dyn Error>> {
        let reader = NatsEventReader::new(nats_client.clone()).await?;
        let writer = NatsCommandWriter::new(nats_client).await?;
        let engine = ScriptEngine::new(script_path);

        Ok(Self {
            reader,
            writer,
            engine,
        })
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        tracing::info!("started game loop");

        while let Some(message) = self.reader.read_message().await? {
            tracing::info!(message = %message.subject, "received new message");

            if let Err(error) = self.engine.handle_event(&message.subject, &message.payload) {
                tracing::error!(error = %error, "failed to handle event");
            };

            ack_message(message.acker).await?;
        }

        Ok(())
    }
}

async fn ack_message(acker: Acker) -> Result<(), Box<dyn Error>> {
    acker
        .ack()
        .await
        .map_err(|error| -> Box<dyn Error> { error })?;
    tracing::debug!("acked game event");

    Ok(())
}
