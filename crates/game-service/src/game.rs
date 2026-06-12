use crate::engine;
use nexus_shared::nats::{MessageReader, NatsAdapter, NatsCommandMessage};
use std::error::Error;

const EVENTS_CONSUMER: &str = "game-service";

pub struct Game {
    nats: NatsAdapter,
    events: MessageReader,
    engine: engine::ScriptEngine,
}

impl Game {
    pub async fn new(nats: NatsAdapter, script_path: &str) -> Result<Self, Box<dyn Error>> {
        let events = nats.events_reader(EVENTS_CONSUMER).await?;
        let engine = engine::ScriptEngine::new(script_path);

        Ok(Self {
            nats,
            events,
            engine,
        })
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        tracing::info!("started game loop");

        while let Some(message) = self.events.next().await? {
            tracing::info!(message = %message.subject(), "received new message");

            let commands = match self
                .engine
                .handle_event(message.subject(), message.payload())
            {
                Ok(commands) => commands
                    .into_iter()
                    .map(|c| NatsCommandMessage {
                        subject: c.subject,
                        payload: c.payload.into(),
                    })
                    .collect(),
                Err(error) => {
                    tracing::error!(error = %error, "failed to handle event");
                    continue;
                }
            };

            if let Err(error) = self.nats.write_commands(commands).await {
                tracing::error!(error = %error, "failed to write commands");
                continue;
            }

            message.ack().await?;
        }

        Ok(())
    }
}
