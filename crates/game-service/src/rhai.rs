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
        engine.on_print(|message| println!("[game-script] {message}"));

        let ast = engine
            .compile_file(script_path.into())
            .expect("failed to compile script file");

        println!("game-service loaded script from {}", script_path);

        ScriptEngine { engine, ast }
    }

    pub fn handle_event(&self, event: Event) -> HookResult {
        let Some(hook) = mappings::subject_to_hook(&event.subject) else {
            eprintln!("no game hook configured for subject {}", event.subject);
            return Ok(());
        };

        let mut scope = Scope::new();
        let payload: Dynamic = serde_json::from_slice(&event.payload)?;
        self.engine
            .call_fn::<()>(&mut scope, &self.ast, hook, (payload,))?;

        Ok(())
    }
}
