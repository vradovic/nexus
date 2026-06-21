use std::{cell::RefCell, collections::HashMap, rc::Rc};

use rhai::{Array, Blob, Dynamic, EvalAltResult};

pub type StateStore = Rc<RefCell<HashMap<String, Dynamic>>>;

const BROADCAST_SUBJECT: &str = "commands.broadcast";
const ROOM_BROADCAST_SUBJECT: &str = "commands.broadcast.rooms";
const ROOM_CREATE_SUBJECT: &str = "commands.rooms.create";
const ROOM_REMOVE_SUBJECT: &str = "commands.rooms.remove";
const ROOM_JOIN_SUBJECT: &str = "commands.rooms.join";
const ROOM_LEAVE_SUBJECT: &str = "commands.rooms.leave";

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

    pub fn broadcast_to_rooms(
        &mut self,
        message: Dynamic,
        rooms: Array,
    ) -> Result<(), Box<EvalAltResult>> {
        let rooms = room_ids_from_array(rooms)?;
        let payload = payload_from_dynamic(message)?;
        let command = serde_json::json!({
            "rooms": rooms,
            "payload": payload,
        });
        let payload = serde_json::to_vec(&command)
            .map_err(|error| format!("failed to serialize room broadcast: {error}"))?;

        self.commands.borrow_mut().push(Command {
            subject: ROOM_BROADCAST_SUBJECT.to_string(),
            payload,
        });
        Ok(())
    }

    pub fn create_room(&mut self, room: &str) -> Result<(), Box<EvalAltResult>> {
        let room = validate_id(room, "room id")?;
        self.push_json_command(
            ROOM_CREATE_SUBJECT,
            serde_json::json!({
                "room": room,
            }),
        )
    }

    pub fn remove_room(&mut self, room: &str) -> Result<(), Box<EvalAltResult>> {
        let room = validate_id(room, "room id")?;
        self.push_json_command(
            ROOM_REMOVE_SUBJECT,
            serde_json::json!({
                "room": room,
            }),
        )
    }

    pub fn add_to_room(
        &mut self,
        connection_id: &str,
        room: &str,
    ) -> Result<(), Box<EvalAltResult>> {
        let connection_id = validate_id(connection_id, "connection id")?;
        let room = validate_id(room, "room id")?;
        self.push_json_command(
            ROOM_JOIN_SUBJECT,
            serde_json::json!({
                "connection_id": connection_id,
                "room": room,
            }),
        )
    }

    pub fn remove_from_room(
        &mut self,
        connection_id: &str,
        room: &str,
    ) -> Result<(), Box<EvalAltResult>> {
        let connection_id = validate_id(connection_id, "connection id")?;
        let room = validate_id(room, "room id")?;
        self.push_json_command(
            ROOM_LEAVE_SUBJECT,
            serde_json::json!({
                "connection_id": connection_id,
                "room": room,
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

fn room_ids_from_array(rooms: Array) -> Result<Vec<String>, Box<EvalAltResult>> {
    rooms
        .into_iter()
        .map(|room| {
            room.try_cast::<String>()
                .filter(|room| !room.is_empty())
                .ok_or_else(|| "room id must be a non-empty string".into())
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
