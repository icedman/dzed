pub mod actions;
pub mod command;
pub mod controllers;
pub mod ex;
pub mod exmap;
pub mod input;
pub mod keymap;
pub mod macros;

use crate::controller::controllers::ViewController;
use crate::controller::controllers::textview::TextViewController;
use crate::ui::views::View;
use crate::{controller::input::VimInput, editor, ui::Ui, ui::window};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use std::collections::VecDeque;

pub enum ControllerResult {
    None,
    Exit,
}

pub struct Controller {
    pub input: VimInput,
    pub command: command::Command,
    pub keymap: keymap::Keymap,
    pub macro_recorder: macros::MacroRecorder,
    pub pending_actions: VecDeque<actions::Action>,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            input: input::VimInput::new(),
            command: command::Command::new(),
            keymap: keymap::Keymap::new(),
            macro_recorder: macros::MacroRecorder::new(),
            pending_actions: VecDeque::new(),
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
                let action = self.input.handle_event(&key_event);
                match action {
                    actions::Action::NoOp => {}
                    any => {
                        self.pending_actions.push_back(any.clone());
                    }
                }
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

    pub fn dispatch_actions(
        &mut self,
        editor: &mut crate::editor::Editor,
        ui: &crate::ui::Ui,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        let mut last_result = ControllerResult::None;

        while let Some(action) = self.pending_actions.pop_front() {
            editor.last_action = action.clone();
            if let Some(window) = ui.get_focused_window() {
                if let Some(ref controller) = window.controller {
                    last_result = controller.handle_action(action, editor, ui, window.id)?;
                }
            }
        }

        Ok(last_result)
    }
}
