use crate::{
    engine,
    nats::{self, NatsCommandMessage},
};
use std::error::Error;

pub struct Game {
    nats: nats::NatsAdapter,
    engine: engine::ScriptEngine,
}

impl Game {
    pub async fn new(
        nats_client: async_nats::Client,
        script_path: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let nats = nats::NatsAdapter::new(nats_client).await?;
        let engine = engine::ScriptEngine::new(script_path);

        Ok(Self { nats, engine })
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        tracing::info!("started game loop");

        while let Some(message) = self.nats.read_events().await? {
            tracing::info!(message = %message.subject, "received new message");

            let commands = match self.engine.handle_event(&message.subject, &message.payload) {
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

            nats::ack_message(message.acker).await?;
        }

        Ok(())
    }
}
