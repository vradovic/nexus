use bytes::Bytes;
use rhai::{AST, Engine, EvalAltResult, Scope};

pub const DEFAULT_SCRIPT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/game.rhai");
const HELLO_WORLD_SUBJECT: &str = "events.hello_world";
const FOO_SUBJECT: &str = "events.foo";

pub struct Event {
    pub subject: String,
    pub payload: Bytes,
}

pub struct ScriptEngine {
    engine: Engine,
    ast: AST,
}

pub type HookResult = Result<(), Box<EvalAltResult>>;

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
        match event.subject.as_str() {
            HELLO_WORLD_SUBJECT => self.on_hello_world(event),
            FOO_SUBJECT => self.on_foo(),
            unknown_subject => {
                eprintln!("no game hook configured for subject '{}'", unknown_subject);
                Ok(())
            }
        }
    }

    fn on_hello_world(&self, event: Event) -> HookResult {
        let mut scope = Scope::new();
        let name = String::from_utf8_lossy(&event.payload).trim().to_string();
        let result =
            self.engine
                .call_fn::<String>(&mut scope, &self.ast, "on_hello_world", (name,))?;

        println!("[game-script] {result}");

        Ok(())
    }

    fn on_foo(&self) -> HookResult {
        let mut scope = Scope::new();
        let result = self
            .engine
            .call_fn::<String>(&mut scope, &self.ast, "on_foo", ())?;

        println!("[game-script] {result}");

        Ok(())
    }
}
