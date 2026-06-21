use rhai::{AST, Dynamic, Engine, Scope};

use crate::{
    commands::{Command, CommandApi, StateStore},
    mappings,
};

pub const DEFAULT_SCRIPT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/game.rhai");

pub struct ScriptEngine {
    engine: Engine,
    ast: AST,
    state: StateStore,
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
            .register_fn("broadcast", CommandApi::broadcast)
            .register_fn("put", CommandApi::put)
            .register_fn("get", CommandApi::get);

        ScriptEngine {
            engine,
            ast,
            state: Default::default(),
        }
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

        let api = CommandApi::new(self.state.clone());

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

#[cfg(test)]
mod tests {
    use super::*;

    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn state_store_persists_between_events() {
        let script_path = write_script(
            r#"
                fn on_message(payload, api) {
                    if api.get("count") == () {
                        api.put("count", 0);
                    }

                    let count = api.get("count") + 1;
                    api.put("count", count);
                    api.broadcast(count);
                }
            "#,
        );

        let mut engine =
            ScriptEngine::new(script_path.to_str().expect("script path is valid UTF-8"));

        let first_commands = engine
            .handle_event("events.realtime.message", &[])
            .expect("first event is handled");
        let second_commands = engine
            .handle_event("events.realtime.message", &[])
            .expect("second event is handled");

        fs::remove_file(script_path).ok();

        assert_eq!(first_commands.len(), 1);
        assert_eq!(first_commands[0].payload, b"1");
        assert_eq!(second_commands.len(), 1);
        assert_eq!(second_commands[0].payload, b"2");
    }

    fn write_script(contents: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "game-service-state-test-{}-{timestamp}.rhai",
            std::process::id()
        ));

        fs::write(&path, contents).expect("test script is written");
        path
    }
}
