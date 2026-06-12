use std::{cell::RefCell, rc::Rc};

use rhai::EvalAltResult;

#[derive(Clone)]
pub struct Command {
    pub subject: String,
    pub payload: Vec<u8>,
}

#[derive(Default, Clone)]
pub struct CommandApi {
    commands: Rc<RefCell<Vec<Command>>>,
}

impl CommandApi {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn broadcast(&mut self, message: rhai::Dynamic) -> Result<(), Box<EvalAltResult>> {
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

    pub fn take_commands(&self) -> Vec<Command> {
        std::mem::take(&mut self.commands.borrow_mut())
    }
}
