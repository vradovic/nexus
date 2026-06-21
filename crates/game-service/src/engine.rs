use rhai::{AST, Array, Dynamic, Engine, Map, Scope};
use serde::Deserialize;

use crate::{
    commands::{Command, CommandApi, StateStore},
    mappings,
};

pub const DEFAULT_SCRIPT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/game.rhai");
const REALTIME_MESSAGE_EVENT_SUBJECT: &str = "events.realtime.message";

#[derive(Debug, Deserialize)]
struct RealtimeMessageEvent {
    connection_id: String,
    rooms: Vec<String>,
    #[serde(default)]
    all_rooms: Vec<String>,
    payload: Vec<u8>,
}

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
            .register_fn("broadcast_to_rooms", CommandApi::broadcast_to_rooms)
            .register_fn("create_room", CommandApi::create_room)
            .register_fn("remove_room", CommandApi::remove_room)
            .register_fn("add_to_room", CommandApi::add_to_room)
            .register_fn("remove_from_room", CommandApi::remove_from_room)
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

        let payload = payload_to_dynamic(subject, payload);

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

fn payload_to_dynamic(subject: &str, payload: &[u8]) -> Dynamic {
    if subject == REALTIME_MESSAGE_EVENT_SUBJECT {
        if let Ok(event) = serde_json::from_slice::<RealtimeMessageEvent>(payload) {
            return realtime_message_to_dynamic(event);
        }
    }

    Dynamic::from_blob(payload.to_vec())
}

fn realtime_message_to_dynamic(event: RealtimeMessageEvent) -> Dynamic {
    let mut map = Map::new();
    let rooms = event
        .rooms
        .into_iter()
        .map(Dynamic::from)
        .collect::<Array>();
    let all_rooms = event
        .all_rooms
        .into_iter()
        .map(Dynamic::from)
        .collect::<Array>();

    map.insert("connection_id".into(), Dynamic::from(event.connection_id));
    map.insert("rooms".into(), Dynamic::from_array(rooms));
    map.insert("all_rooms".into(), Dynamic::from_array(all_rooms));
    map.insert("payload".into(), Dynamic::from_blob(event.payload));

    Dynamic::from_map(map)
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

    #[test]
    fn realtime_event_exposes_all_rooms_to_scripts() {
        let script_path = write_script(
            r#"
                fn on_message(event, api) {
                    api.broadcast_to_rooms(event.payload, event.all_rooms);
                }
            "#,
        );
        let mut engine =
            ScriptEngine::new(script_path.to_str().expect("script path is valid UTF-8"));
        let event = serde_json::json!({
            "connection_id": "connection-1",
            "rooms": ["match-1"],
            "all_rooms": ["default", "match-1"],
            "payload": [104, 105],
        });

        let commands = engine
            .handle_event(
                "events.realtime.message",
                &serde_json::to_vec(&event).expect("event is serialized"),
            )
            .expect("event is handled");

        fs::remove_file(script_path).ok();

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].subject, "commands.broadcast.rooms");

        let command =
            serde_json::from_slice::<serde_json::Value>(&commands[0].payload).expect("valid json");
        assert_eq!(command["rooms"], serde_json::json!(["default", "match-1"]));
        assert_eq!(command["payload"], serde_json::json!([104, 105]));
    }

    #[test]
    fn room_management_api_queues_room_commands() {
        let script_path = write_script(
            r#"
                fn on_message(event, api) {
                    api.create_room("match-2");
                    api.add_to_room(event.connection_id, "match-2");
                    api.remove_from_room(event.connection_id, "default");
                    api.remove_room("match-1");
                }
            "#,
        );
        let mut engine =
            ScriptEngine::new(script_path.to_str().expect("script path is valid UTF-8"));
        let connection_id = "550e8400-e29b-41d4-a716-446655440000";
        let event = serde_json::json!({
            "connection_id": connection_id,
            "rooms": ["default"],
            "all_rooms": ["default", "match-1"],
            "payload": [],
        });

        let commands = engine
            .handle_event(
                "events.realtime.message",
                &serde_json::to_vec(&event).expect("event is serialized"),
            )
            .expect("event is handled");

        fs::remove_file(script_path).ok();

        assert_eq!(commands.len(), 4);
        assert_eq!(commands[0].subject, "commands.rooms.create");
        assert_eq!(commands[1].subject, "commands.rooms.join");
        assert_eq!(commands[2].subject, "commands.rooms.leave");
        assert_eq!(commands[3].subject, "commands.rooms.remove");

        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&commands[0].payload)
                .expect("create room command is valid"),
            serde_json::json!({ "room": "match-2" })
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&commands[1].payload)
                .expect("join room command is valid"),
            serde_json::json!({ "connection_id": connection_id, "room": "match-2" })
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&commands[2].payload)
                .expect("leave room command is valid"),
            serde_json::json!({ "connection_id": connection_id, "room": "default" })
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&commands[3].payload)
                .expect("remove room command is valid"),
            serde_json::json!({ "room": "match-1" })
        );
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
