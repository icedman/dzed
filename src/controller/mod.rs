use crate::controller::input::VimInput;

pub mod actions;
pub mod command;
pub mod ex;
pub mod exmap;
pub mod input;
pub mod keymap;
pub mod macros;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind};

pub enum ControllerResult {
    None,
    Exit,
}

pub struct Controller {
    pub input: VimInput,
    pub command: command::Command,
    pub keymap: keymap::Keymap,
    pub macro_recorder: macros::MacroRecorder,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            input: input::VimInput::new(),
            command: command::Command::new(),
            keymap: keymap::Keymap::new(),
            macro_recorder: macros::MacroRecorder::new(),
        }
    }

    pub fn handle_event(
        &mut self,
        event: crossterm::event::Event,
        editor: &mut crate::editor::Editor,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        match event {
            Event::Key(key_event) => {
                self.input.set_mode(editor.mode);
                let mut action = self.input.handle_event(&key_event);

                match (key_event.code, key_event.modifiers) {
                    (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                        return Ok(ControllerResult::Exit);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(ControllerResult::None)
    }
}
