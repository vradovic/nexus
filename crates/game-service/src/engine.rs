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
    channels: Vec<String>,
    #[serde(default)]
    all_channels: Vec<String>,
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
            .register_fn("broadcast_to_channels", CommandApi::broadcast_to_channels)
            .register_fn("create_channel", CommandApi::create_channel)
            .register_fn("remove_channel", CommandApi::remove_channel)
            .register_fn("add_to_channel", CommandApi::add_to_channel)
            .register_fn("remove_from_channel", CommandApi::remove_from_channel)
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

    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(payload) {
        if let Ok(dynamic) = rhai::serde::to_dynamic(value) {
            return dynamic;
        }
    }

    Dynamic::from_blob(payload.to_vec())
}

fn realtime_message_to_dynamic(event: RealtimeMessageEvent) -> Dynamic {
    let mut map = Map::new();
    let channels = event
        .channels
        .into_iter()
        .map(Dynamic::from)
        .collect::<Array>();
    let all_channels = event
        .all_channels
        .into_iter()
        .map(Dynamic::from)
        .collect::<Array>();

    map.insert("connection_id".into(), Dynamic::from(event.connection_id));
    map.insert("channels".into(), Dynamic::from_array(channels));
    map.insert("all_channels".into(), Dynamic::from_array(all_channels));
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
    fn realtime_event_exposes_all_channels_to_scripts() {
        let script_path = write_script(
            r#"
                fn on_message(event, api) {
                    api.broadcast_to_channels(event.payload, event.all_channels);
                }
            "#,
        );
        let mut engine =
            ScriptEngine::new(script_path.to_str().expect("script path is valid UTF-8"));
        let event = serde_json::json!({
            "connection_id": "connection-1",
            "channels": ["match-1"],
            "all_channels": ["default", "match-1"],
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
        assert_eq!(commands[0].subject, "commands.broadcast.channels");

        let command =
            serde_json::from_slice::<serde_json::Value>(&commands[0].payload).expect("valid json");
        assert_eq!(
            command["channels"],
            serde_json::json!(["default", "match-1"])
        );
        assert_eq!(command["payload"], serde_json::json!([104, 105]));
    }

    #[test]
    fn channel_management_api_queues_channel_commands() {
        let script_path = write_script(
            r#"
                fn on_message(event, api) {
                    api.create_channel("match-2");
                    api.add_to_channel(event.connection_id, "match-2");
                    api.remove_from_channel(event.connection_id, "default");
                    api.remove_channel("match-1");
                }
            "#,
        );
        let mut engine =
            ScriptEngine::new(script_path.to_str().expect("script path is valid UTF-8"));
        let connection_id = "550e8400-e29b-41d4-a716-446655440000";
        let event = serde_json::json!({
            "connection_id": connection_id,
            "channels": ["default"],
            "all_channels": ["default", "match-1"],
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
        assert_eq!(commands[0].subject, "commands.channels.create");
        assert_eq!(commands[1].subject, "commands.channels.join");
        assert_eq!(commands[2].subject, "commands.channels.leave");
        assert_eq!(commands[3].subject, "commands.channels.remove");

        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&commands[0].payload)
                .expect("create channel command is valid"),
            serde_json::json!({ "channel": "match-2" })
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&commands[1].payload)
                .expect("join channel command is valid"),
            serde_json::json!({ "connection_id": connection_id, "channel": "match-2" })
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&commands[2].payload)
                .expect("leave channel command is valid"),
            serde_json::json!({ "connection_id": connection_id, "channel": "default" })
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&commands[3].payload)
                .expect("remove channel command is valid"),
            serde_json::json!({ "channel": "match-1" })
        );
    }

    #[test]
    fn match_confirmed_event_can_create_channel_and_add_players() {
        let script_path = write_script(
            r#"
                fn on_match_confirmed(event, api) {
                    let channel = "match:" + event.match_id;

                    api.create_channel(channel);

                    for player_id in event.player_ids {
                        api.add_to_channel(player_id, channel);
                    }
                }
            "#,
        );
        let mut engine =
            ScriptEngine::new(script_path.to_str().expect("script path is valid UTF-8"));
        let event = serde_json::json!({
            "match_id": "11111111-1111-1111-1111-111111111111",
            "rule_id": "22222222-2222-2222-2222-222222222222",
            "ticket_key": "duel",
            "player_ids": [
                "33333333-3333-3333-3333-333333333333",
                "44444444-4444-4444-4444-444444444444"
            ],
        });

        let commands = engine
            .handle_event(
                nexus_shared::MATCH_CONFIRMED_SUBJECT,
                &serde_json::to_vec(&event).expect("event is serialized"),
            )
            .expect("event is handled");

        fs::remove_file(script_path).ok();

        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].subject, "commands.channels.create");
        assert_eq!(commands[1].subject, "commands.channels.join");
        assert_eq!(commands[2].subject, "commands.channels.join");

        let channel = "match:11111111-1111-1111-1111-111111111111";
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&commands[0].payload)
                .expect("create channel command is valid"),
            serde_json::json!({ "channel": channel })
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&commands[1].payload)
                .expect("join channel command is valid"),
            serde_json::json!({
                "connection_id": "33333333-3333-3333-3333-333333333333",
                "channel": channel,
            })
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&commands[2].payload)
                .expect("join channel command is valid"),
            serde_json::json!({
                "connection_id": "44444444-4444-4444-4444-444444444444",
                "channel": channel,
            })
        );
    }

    #[test]
    fn default_match_confirmed_script_notifies_match_channel() {
        let mut engine = ScriptEngine::new(DEFAULT_SCRIPT_PATH);
        let event = serde_json::json!({
            "match_id": "11111111-1111-1111-1111-111111111111",
            "rule_id": "22222222-2222-2222-2222-222222222222",
            "ticket_key": "duel",
            "player_ids": [
                "33333333-3333-3333-3333-333333333333",
                "44444444-4444-4444-4444-444444444444"
            ],
        });

        let commands = engine
            .handle_event(
                nexus_shared::MATCH_CONFIRMED_SUBJECT,
                &serde_json::to_vec(&event).expect("event is serialized"),
            )
            .expect("event is handled");

        assert_eq!(commands.len(), 6);
        assert_eq!(commands[0].subject, "commands.channels.create");
        assert_eq!(commands[1].subject, "commands.channels.join");
        assert_eq!(commands[2].subject, "commands.channels.leave");
        assert_eq!(commands[3].subject, "commands.channels.join");
        assert_eq!(commands[4].subject, "commands.channels.leave");
        assert_eq!(commands[5].subject, "commands.broadcast.channels");

        let channel = "match:11111111-1111-1111-1111-111111111111";
        let broadcast = serde_json::from_slice::<serde_json::Value>(&commands[5].payload)
            .expect("broadcast command is valid json");
        assert_eq!(broadcast["channels"], serde_json::json!([channel]));

        let payload = broadcast["payload"]
            .as_array()
            .expect("broadcast payload is a byte array")
            .iter()
            .map(|value| {
                value
                    .as_u64()
                    .and_then(|byte| u8::try_from(byte).ok())
                    .expect("payload byte is valid")
            })
            .collect::<Vec<_>>();
        let message =
            serde_json::from_slice::<serde_json::Value>(&payload).expect("message is valid json");

        assert_eq!(message["type"], "match.found");
        assert_eq!(message["match_id"], "11111111-1111-1111-1111-111111111111");
        assert_eq!(message["channel"], channel);
        assert_eq!(message["ticket_key"], "duel");
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
