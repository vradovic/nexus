use bytes::Bytes;
use rhai::{AST, Dynamic, Engine, Scope};

use crate::mappings;

pub const DEFAULT_SCRIPT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/game.rhai");

pub struct Event {
    pub subject: String,
    pub payload: Bytes,
}

pub struct ScriptEngine {
    engine: Engine,
    ast: AST,
}

pub type HookResult = Result<(), Box<dyn std::error::Error>>;

impl ScriptEngine {
    pub fn new(script_path: &str) -> Self {
        let mut engine = Engine::new();
        engine.on_print(|message| tracing::info!(target: "game_script", %message));

        let ast = engine
            .compile_file(script_path.into())
            .expect("failed to compile script file");

        tracing::info!(script_path, "loaded game script");

        ScriptEngine { engine, ast }
    }

    pub fn handle_event(&self, event: Event) -> HookResult {
        let Some(hook) = mappings::subject_to_hook(&event.subject) else {
            tracing::warn!(subject = %event.subject, "no game hook configured");
            return Ok(());
        };

        let mut scope = Scope::new();
        tracing::debug!(
            subject = %event.subject,
            hook,
            payload_size = event.payload.len(),
            payload = %String::from_utf8_lossy(&event.payload),
            "deserializing game event payload"
        );

        let payload: Dynamic = serde_json::from_slice(&event.payload)?;
        tracing::debug!(subject = %event.subject, hook, "calling game hook");
        self.engine
            .call_fn::<()>(&mut scope, &self.ast, hook, (payload,))?;

        tracing::info!(subject = %event.subject, hook, "handled game event");

        Ok(())
    }
}
