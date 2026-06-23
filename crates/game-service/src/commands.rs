use std::{cell::RefCell, collections::HashMap, rc::Rc};

use rhai::{Array, Blob, Dynamic, EvalAltResult};

pub type StateStore = Rc<RefCell<HashMap<String, Dynamic>>>;

const BROADCAST_SUBJECT: &str = "commands.broadcast";
const CHANNEL_BROADCAST_SUBJECT: &str = "commands.broadcast.channels";
const CHANNEL_CREATE_SUBJECT: &str = "commands.channels.create";
const CHANNEL_REMOVE_SUBJECT: &str = "commands.channels.remove";
const CHANNEL_JOIN_SUBJECT: &str = "commands.channels.join";
const CHANNEL_LEAVE_SUBJECT: &str = "commands.channels.leave";

#[derive(Clone)]
pub struct Command {
    pub subject: String,
    pub payload: Vec<u8>,
}

#[derive(Clone)]
pub struct CommandApi {
    commands: Rc<RefCell<Vec<Command>>>,
    state: StateStore,
}

impl CommandApi {
    pub fn new(state: StateStore) -> Self {
        Self {
            commands: Default::default(),
            state,
        }
    }

    pub fn broadcast(&mut self, message: Dynamic) -> Result<(), Box<EvalAltResult>> {
        let payload = payload_from_dynamic(message)?;

        self.commands.borrow_mut().push(Command {
            subject: BROADCAST_SUBJECT.to_string(),
            payload,
        });
        Ok(())
    }

    pub fn broadcast_to_channels(
        &mut self,
        message: Dynamic,
        channels: Array,
    ) -> Result<(), Box<EvalAltResult>> {
        let channels = channel_ids_from_array(channels)?;
        let payload = payload_from_dynamic(message)?;
        let command = serde_json::json!({
            "channels": channels,
            "payload": payload,
        });
        let payload = serde_json::to_vec(&command)
            .map_err(|error| format!("failed to serialize channel broadcast: {error}"))?;

        self.commands.borrow_mut().push(Command {
            subject: CHANNEL_BROADCAST_SUBJECT.to_string(),
            payload,
        });
        Ok(())
    }

    pub fn create_channel(&mut self, channel: &str) -> Result<(), Box<EvalAltResult>> {
        let channel = validate_id(channel, "channel id")?;
        self.push_json_command(
            CHANNEL_CREATE_SUBJECT,
            serde_json::json!({
                "channel": channel,
            }),
        )
    }

    pub fn remove_channel(&mut self, channel: &str) -> Result<(), Box<EvalAltResult>> {
        let channel = validate_id(channel, "channel id")?;
        self.push_json_command(
            CHANNEL_REMOVE_SUBJECT,
            serde_json::json!({
                "channel": channel,
            }),
        )
    }

    pub fn add_to_channel(
        &mut self,
        connection_id: &str,
        channel: &str,
    ) -> Result<(), Box<EvalAltResult>> {
        let connection_id = validate_id(connection_id, "connection id")?;
        let channel = validate_id(channel, "channel id")?;
        self.push_json_command(
            CHANNEL_JOIN_SUBJECT,
            serde_json::json!({
                "connection_id": connection_id,
                "channel": channel,
            }),
        )
    }

    pub fn remove_from_channel(
        &mut self,
        connection_id: &str,
        channel: &str,
    ) -> Result<(), Box<EvalAltResult>> {
        let connection_id = validate_id(connection_id, "connection id")?;
        let channel = validate_id(channel, "channel id")?;
        self.push_json_command(
            CHANNEL_LEAVE_SUBJECT,
            serde_json::json!({
                "connection_id": connection_id,
                "channel": channel,
            }),
        )
    }

    pub fn put(&mut self, key: &str, value: Dynamic) {
        self.state.borrow_mut().insert(key.to_string(), value);
    }

    pub fn get(&mut self, key: &str) -> Dynamic {
        self.state
            .borrow()
            .get(key)
            .cloned()
            .unwrap_or(Dynamic::UNIT)
    }

    pub fn take_commands(&self) -> Vec<Command> {
        std::mem::take(&mut self.commands.borrow_mut())
    }

    fn push_json_command(
        &mut self,
        subject: &str,
        payload: serde_json::Value,
    ) -> Result<(), Box<EvalAltResult>> {
        let payload = serde_json::to_vec(&payload)
            .map_err(|error| format!("failed to serialize command: {error}"))?;

        self.commands.borrow_mut().push(Command {
            subject: subject.to_string(),
            payload,
        });

        Ok(())
    }
}

fn payload_from_dynamic(message: Dynamic) -> Result<Vec<u8>, Box<EvalAltResult>> {
    if let Some(blob) = message.clone().try_cast::<Blob>() {
        return Ok(blob);
    }

    let val = rhai::serde::from_dynamic::<serde_json::Value>(&message)?;

    serde_json::to_vec(&val).map_err(|error| format!("failed to serialize payload: {error}").into())
}

fn channel_ids_from_array(channels: Array) -> Result<Vec<String>, Box<EvalAltResult>> {
    channels
        .into_iter()
        .map(|channel| {
            channel
                .try_cast::<String>()
                .filter(|channel| !channel.is_empty())
                .ok_or_else(|| "channel id must be a non-empty string".into())
        })
        .collect()
}

fn validate_id(value: &str, label: &str) -> Result<String, Box<EvalAltResult>> {
    let value = value.trim();

    if value.is_empty() {
        return Err(format!("{label} must be a non-empty string").into());
    }

    Ok(value.to_string())
}
