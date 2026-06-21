use std::{cell::RefCell, collections::HashMap, rc::Rc};

use rhai::{Dynamic, EvalAltResult};

pub type StateStore = Rc<RefCell<HashMap<String, Dynamic>>>;

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
        let subject = "commands.broadcast".to_string();

        let payload = if let Some(blob) = message.clone().try_cast::<rhai::Blob>() {
            blob
        } else {
            let val = rhai::serde::from_dynamic::<serde_json::Value>(&message)?;

            serde_json::to_vec(&val)
                .map_err(|error| format!("failed to serialize payload: {error}"))?
        };

        self.commands
            .borrow_mut()
            .push(Command { subject, payload });
        Ok(())
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
}
