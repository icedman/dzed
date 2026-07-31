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
use crate::services::background;
use crate::ui::views::View;
use crate::{controller::input::VimInput, editor, ui::Ui, ui::window};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub enum ControllerResult {
    None,
    Exit,
    Action(actions::Action),
    Command(String),
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
        buffer_manager: &mut crate::editor::buffers::BufferManager,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        match event {
            Event::Key(key_event) => {
                self.input.set_mode(editor.mode);
                self.input.is_macro_recording = self.macro_recorder.is_recording();
                let action = self.input.handle_event(&key_event);
                editor.pending_keys = self.input.pending_keys_str();
                match action {
                    actions::Action::NoOp => {}
                    any => {
                        self.pending_actions.push_back(any.clone());
                    }
                }
                // match (key_event.code, key_event.modifiers) {
                //     (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                //         return Ok(ControllerResult::Exit);
                //     }
                //     _ => {}
                // }
            }
            _ => {}
        }
        Ok(ControllerResult::None)
    }

    pub fn dispatch_actions(
        &mut self,
        editor: &mut crate::editor::Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
        ui: &mut crate::ui::Ui,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        let mut last_result = ControllerResult::None;

        while let Some(action) = self.pending_actions.pop_front() {
            match &action {
                actions::Action::BeginMacro { register } => {
                    self.macro_recorder.begin(register.clone());
                }
                actions::Action::EndMacro => {
                    self.macro_recorder.end();
                }
                actions::Action::ReplayMacro { register, count } => {
                    if let Some(macro_actions) = self.macro_recorder.get(register) {
                        let actions_to_replay = macro_actions.clone();
                        for _ in 0..*count {
                            for act in actions_to_replay.iter().rev() {
                                self.pending_actions.push_front(act.clone());
                            }
                        }
                    }
                }
                actions::Action::FocusLeftWindow => {
                    if let Some(nid) =
                        ui.find_neighbor(crate::ui::layout::NavigationDirection::Left)
                    {
                        ui.set_focused_window(nid);
                        editor.should_redraw = true;
                    }
                }
                actions::Action::FocusDownWindow => {
                    if let Some(nid) =
                        ui.find_neighbor(crate::ui::layout::NavigationDirection::Down)
                    {
                        ui.set_focused_window(nid);
                        editor.should_redraw = true;
                    }
                }
                actions::Action::FocusUpWindow => {
                    if let Some(nid) = ui.find_neighbor(crate::ui::layout::NavigationDirection::Up)
                    {
                        ui.set_focused_window(nid);
                        editor.should_redraw = true;
                    }
                }
                actions::Action::FocusRightWindow => {
                    if let Some(nid) =
                        ui.find_neighbor(crate::ui::layout::NavigationDirection::Right)
                    {
                        ui.set_focused_window(nid);
                        editor.should_redraw = true;
                    }
                }
                actions::Action::Command(command_string) => {
                    self.command.set(command_string);
                    if let Some(result) = self.command.ex(ui, editor, buffer_manager) {
                        editor.should_redraw = true;
                        last_result = result;
                    }
                }
                _ => {
                    self.macro_recorder.update(&action);

                    editor.last_action = action.clone();

                    match action {
                        actions::Action::SetToCommand => {
                            ui.focus_window(crate::ui::WindowId::CommandLine as usize);
                            editor.should_redraw = true;
                        }
                        actions::Action::SearchForward { .. } => {
                            ui.focus_window(crate::ui::WindowId::CommandLine as usize);
                            editor.should_redraw = true;
                        }
                        actions::Action::SearchBackward { .. } => {
                            ui.focus_window(crate::ui::WindowId::CommandLine as usize);
                            editor.should_redraw = true;
                        }
                        _ => {}
                    };

                    let old_mode = editor.mode;
                    let focused_id = ui.focused_window_id;
                    if let Some(window_id) = focused_id {
                        let mut controller = ui
                            .windows
                            .get_mut(&window_id)
                            .and_then(|w| w.controller.take());
                        if let Some(ref mut c) = controller {
                            last_result =
                                c.handle_action(action, editor, buffer_manager, ui, window_id)?;
                        }
                        if let Some(w) = ui.windows.get_mut(&window_id) {
                            w.controller = controller;
                        }
                    }

                    if old_mode == actions::Mode::Command && editor.mode != actions::Mode::Command {
                        ui.restore_last_focused_window();
                        editor.should_redraw = true;
                    }
                }
            }

            match last_result {
                ControllerResult::Command(ref cmd_text) => {
                    self.pending_actions.push_back(actions::Action::SetToNormal);
                    self.pending_actions
                        .push_back(actions::Action::Command(cmd_text.clone()));
                }
                ControllerResult::Action(ref act) => {
                    self.pending_actions.push_back(actions::Action::SetToNormal);
                    self.pending_actions.push_back(act.clone());
                }
                _ => {}
            }
        }

        editor.pending_keys = self.input.pending_keys_str();
        Ok(last_result)
    }
}
