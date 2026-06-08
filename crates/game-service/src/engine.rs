use rhai::{AST, Dynamic, Engine, Scope};

use crate::mappings;

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

        ScriptEngine { engine, ast }
    }

    pub fn handle_event(
        &self,
        subject: &str,
        payload: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(hook) = mappings::subject_to_hook(subject) else {
            tracing::warn!(subject = %subject, "no game hook configured");
            return Ok(());
        };

        let mut scope = Scope::new();

        tracing::debug!(
            subject = %subject,
            hook,
            payload_size = payload.len(),
            payload = %String::from_utf8_lossy(payload),
            "deserializing game event payload"
        );

        let payload: Dynamic = serde_json::from_slice(payload)?;

        tracing::debug!(subject = %subject, hook, "calling game hook");

        self.engine
            .call_fn::<()>(&mut scope, &self.ast, hook, (payload,))?;

        tracing::info!(
            subject = %subject,
            hook,
            "handled game event"
        );

        Ok(())
    }
}
