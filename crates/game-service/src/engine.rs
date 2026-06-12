use rhai::{AST, Dynamic, Engine, Scope};

use crate::{
    commands::{Command, CommandApi},
    mappings,
};

pub const DEFAULT_SCRIPT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/game.rhai");

pub struct ScriptEngine {
    engine: Engine,
    ast: AST,
}

impl ScriptEngine {
    pub fn new(script_path: &str) -> Self {
        let mut engine = Engine::new();
        engine.on_print(|message| tracing::info!(print=%message));

        let ast = engine
            .compile_file(script_path.into())
            .expect("failed to compile script file");

        tracing::info!(script_path, "loaded game script");

        engine
            .register_type::<CommandApi>()
            .register_fn("broadcast", CommandApi::broadcast);

        ScriptEngine { engine, ast }
    }

    pub fn handle_event(
        &mut self,
        subject: &str,
        payload: &[u8],
    ) -> Result<Vec<Command>, Box<dyn std::error::Error>> {
        let Some(hook) = mappings::subject_to_hook(subject) else {
            tracing::warn!(subject = %subject, "no game hook configured");
            return Ok(vec![]);
        };

        let mut scope = Scope::new();

        tracing::debug!(
            subject = %subject,
            hook,
            payload_size = payload.len(),
            "calling game hook with binary payload"
        );

        let payload = Dynamic::from_blob(payload.to_vec());

        tracing::debug!(subject = %subject, hook, "calling game hook");

        let api = CommandApi::new();

        self.engine
            .call_fn::<()>(&mut scope, &self.ast, hook, (payload, api.clone()))?;

        let commands = api.take_commands();

        tracing::info!(
            subject = %subject,
            hook,
            command_count = commands.len(),
            "handled game event"
        );

        Ok(commands)
    }
}
