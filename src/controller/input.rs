use crate::controller;
use crate::controller::actions::{Action, Mode};
use crate::controller::keymap::{InputStateMachine, KeyCombo, Keymap};
use crate::editor::document::BufferText;
use crate::services::clipboard;
use crossterm::event::{Event, KeyCode, KeyEvent, MouseEventKind};

pub enum HandleEvent {
    Redraw,
    RedrawAndSync,
    NoRedraw,
    Exit,
}

// this is global handler
pub fn handle_event(editor: &mut crate::editor::Editor, event: Event) -> HandleEvent {
    match event {
        Event::Key(key_event) => {
            let action = if editor.controller.macro_recorder.is_recording()
                && editor.controller.input.mode() == Mode::Normal
                && key_event.code == crossterm::event::KeyCode::Char('q')
                && key_event.modifiers == crossterm::event::KeyModifiers::NONE
            {
                Action::EndMacro
            } else {
                editor.controller.input.handle_event(&key_event)
            };

            // Sync mode from VimInput to Document
            let active_buffer = editor.buffer_manager.active_mut();
            if active_buffer.doc.current_mode() != editor.controller.input.mode() {
                active_buffer.doc.enter_mode(editor.controller.input.mode());
            }

            if action != Action::NoOp {
                if matches!(action, Action::Quit) {
                    return HandleEvent::Exit;
                }

                // Handle BeginMacro, EndMacro, and ReplayMacro, and update recording
                match &action {
                    Action::BeginMacro { register } => {
                        editor.controller.macro_recorder.begin(register.clone());
                    }
                    Action::EndMacro => {
                        editor.controller.macro_recorder.end();
                    }
                    Action::ReplayMacro { register, count } => {
                        if let Some(macro_actions) = editor.controller.macro_recorder.get(register)
                        {
                            let actions_to_replay = macro_actions.clone();
                            for _ in 0..*count {
                                for act in &actions_to_replay {
                                    if editor.mode == Mode::Command {
                                        editor.apply_command_action(act);
                                    } else {
                                        editor.apply_active_action(act);
                                    }
                                    editor.controller.macro_recorder.update(act);
                                    editor.controller.input.set_mode(
                                        editor.buffer_manager.active().doc.current_mode(),
                                    );
                                    editor.buffer_manager.active_mut().doc.sync();
                                }
                            }
                        }
                    }
                    _ => {
                        editor.controller.macro_recorder.update(&action);
                    }
                }

                let reg_opt = editor.controller.input.last_register;
                if let Some(ch) = reg_opt {
                    if let Some(r_name) = clipboard::RegisterName::from_char(ch) {
                        editor.services.clipboard.borrow_mut().grab(r_name);
                    }
                }

                if editor.mode == Mode::Command {
                    if matches!(action, Action::InsertNewLine { .. }) {
                        let mut command = std::mem::replace(
                            &mut editor.controller.command,
                            controller::command::Command::new(),
                        );
                        let ex_res = command.ex(editor);
                        editor.controller.command = command;
                        if let Some(controller::command::ExResult::Exit) = ex_res {
                            return HandleEvent::Exit;
                        }
                        editor.controller.command.clear();
                        editor.controller.input.set_mode(Mode::Normal);
                        editor
                            .buffer_manager
                            .active_mut()
                            .doc
                            .enter_mode(Mode::Normal);
                        editor.services.clipboard.borrow_mut().release();
                        return HandleEvent::RedrawAndSync;
                    } else if matches!(action, Action::Clear) {
                        editor.controller.command.clear();
                        editor.controller.input.set_mode(Mode::Normal);
                        editor
                            .buffer_manager
                            .active_mut()
                            .doc
                            .enter_mode(Mode::Normal);
                        editor.services.clipboard.borrow_mut().release();
                        return HandleEvent::RedrawAndSync;
                    } else {
                        editor.apply_command_action(&action);
                    }
                } else {
                    match &action {
                        Action::BeginMacro { .. }
                        | Action::EndMacro
                        | Action::ReplayMacro { .. } => {}
                        _ => {
                            editor.apply_active_action(&action);
                        }
                    }
                }

                editor.services.clipboard.borrow_mut().release();

                // After applying action, mode might have changed again
                editor
                    .controller
                    .input
                    .set_mode(editor.buffer_manager.active().doc.current_mode());
                editor.buffer_manager.active_mut().doc.sync();
                return HandleEvent::RedrawAndSync;
            }

            // Handle Esc specifically if it didn't produce an action but should clear state
            if key_event.code == KeyCode::Esc {
                editor.controller.input.clear();
                return HandleEvent::Redraw;
            }

            // If state machine is holding keys, digits, or a pending op, redraw UI status
            if editor.controller.input.is_busy() {
                return HandleEvent::Redraw;
            }
        }
        Event::Mouse(mouse_event) => match mouse_event.kind {
            MouseEventKind::ScrollUp => {
                editor.apply_active_action(&Action::ScrollLineUp { count: 1 });
                return HandleEvent::Redraw;
            }
            MouseEventKind::ScrollDown => {
                editor.apply_active_action(&Action::ScrollLineDown { count: 1 });
                return HandleEvent::Redraw;
            }
            _ => {}
        },
        _ => {}
    }
    HandleEvent::NoRedraw
}

pub struct InputContext {
    pub mode: Mode,
}

impl InputContext {
    pub fn new() -> Self {
        Self { mode: Mode::Normal }
    }
}

pub struct VimInput {
    pub state_machine: InputStateMachine,
    pub keymap: Keymap,
    pub resolved_action: Action,
    pub last_register: Option<char>,
}

impl VimInput {
    pub fn new() -> Self {
        Self {
            state_machine: InputStateMachine::new(),
            keymap: Keymap::new(),
            resolved_action: Action::NoOp,
            last_register: None,
        }
    }

    pub fn mode(&self) -> Mode {
        self.state_machine.mode
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.state_machine.mode = mode;
    }

    /// Returns true if the keymap state machine is holding pending inputs
    /// (digits, partial key sequences, or pending operators).
    pub fn is_busy(&self) -> bool {
        !self.state_machine.count_buffer.is_empty()
            || !self.state_machine.key_sequence.is_empty()
            || self.state_machine.pending_op.is_some()
            || self.state_machine.waiting_for_register
    }

    pub fn clear(&mut self) {
        self.state_machine.clear();
        self.resolved_action = Action::NoOp;
        self.last_register = None;
    }

    pub fn handle_event(&mut self, key_event: &KeyEvent) -> Action {
        // Filter out KeyRelease events from Crossterm to avoid duplicate state transitions
        if key_event.kind == crossterm::event::KeyEventKind::Release {
            return Action::NoOp;
        }

        let combo = KeyCombo::from(key_event);
        let reg_before = self.state_machine.register;

        self.resolved_action = self.state_machine.process_key(combo, &self.keymap);

        if self.resolved_action != Action::NoOp {
            self.last_register = reg_before;
        } else {
            self.last_register = None;
        }

        self.resolved_action.clone()
    }

    pub fn resolved_op(&self) -> Action {
        self.state_machine
            .pending_op
            .clone()
            .unwrap_or(Action::NoOp)
    }

    /// Renders active pending input sequence into a string for editor status bars.
    pub fn pending_keys_str(&self) -> String {
        let mut display = String::new();

        if let Some(ref op) = self.state_machine.pending_op {
            display.push_str(&format!("{}", op));
        }

        if !self.state_machine.count_buffer.is_empty() {
            display.push_str(&self.state_machine.count_buffer);
        }

        for combo in &self.state_machine.key_sequence {
            display.push_str(&combo.to_string());
        }

        display
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn send_key(vim: &mut VimInput, code: KeyCode, modifiers: KeyModifiers) -> Action {
        vim.handle_event(&KeyEvent::new(code, modifiers))
    }

    fn send_char(vim: &mut VimInput, c: char) -> Action {
        if c.is_ascii_uppercase() {
            send_key(vim, KeyCode::Char(c), KeyModifiers::SHIFT)
        } else {
            send_key(vim, KeyCode::Char(c), KeyModifiers::NONE)
        }
    }

    #[test]
    fn test_simple_motions() {
        let mut vim = VimInput::new();

        assert_eq!(
            send_char(&mut vim, 'j'),
            Action::MoveDown {
                count: 1,
                select: false
            }
        );

        send_char(&mut vim, '5');
        assert_eq!(
            send_char(&mut vim, 'k'),
            Action::MoveUp {
                count: 5,
                select: false
            }
        );

        send_char(&mut vim, 'g');
        assert_eq!(
            send_char(&mut vim, 'g'),
            Action::MoveToStartOfDocument {
                count: 1,
                select: false
            }
        );
    }

    #[test]
    fn test_operators() {
        let mut vim = VimInput::new();

        send_char(&mut vim, 'd');
        let action = send_char(&mut vim, 'w');
        if let Action::DeleteMotion { count, motion } = action {
            assert_eq!(count, 1);
            assert_eq!(
                *motion,
                Action::MoveToWord {
                    count: 1,
                    select: false
                }
            );
        } else {
            panic!("Expected DeleteMotion, got {:?}", action);
        }

        send_char(&mut vim, '2');
        send_char(&mut vim, 'd');
        send_char(&mut vim, '3');
        let action = send_char(&mut vim, 'w');
        if let Action::DeleteMotion { count, motion } = action {
            assert_eq!(count, 2);
            assert_eq!(
                *motion,
                Action::MoveToWord {
                    count: 3,
                    select: false
                }
            );
        } else {
            panic!("Expected DeleteMotion, got {:?}", action);
        }
    }

    #[test]
    fn test_line_operators() {
        let mut vim = VimInput::new();

        send_char(&mut vim, 'd');
        assert_eq!(send_char(&mut vim, 'd'), Action::DeleteLine { count: 1 });

        send_char(&mut vim, '5');
        send_char(&mut vim, 'y');
        assert_eq!(send_char(&mut vim, 'y'), Action::YankLine { count: 5 });
    }

    #[test]
    fn test_char_motions() {
        let mut vim = VimInput::new();

        send_char(&mut vim, 'f');
        assert_eq!(
            send_char(&mut vim, 'x'),
            Action::MoveToNextCharacter {
                count: 1,
                select: false,
                till: false,
                ch: 'x'
            }
        );

        send_char(&mut vim, '3');
        send_char(&mut vim, 'F');
        assert_eq!(
            send_char(&mut vim, 'y'),
            Action::MoveToPreviousCharacter {
                count: 3,
                select: false,
                till: false,
                ch: 'y'
            }
        );
    }

    #[test]
    fn test_mode_changes() {
        let mut vim = VimInput::new();

        assert_eq!(vim.mode(), Mode::Normal);
        send_char(&mut vim, 'i');
        assert_eq!(vim.mode(), Mode::Insert);

        send_key(&mut vim, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(vim.mode(), Mode::Normal);

        assert_eq!(send_char(&mut vim, 'a'), Action::SetToAppend);
        assert_eq!(vim.mode(), Mode::Insert);

        send_key(&mut vim, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(vim.mode(), Mode::Normal);

        assert_eq!(send_char(&mut vim, 'A'), Action::SetToAppendEndOfLine);
        assert_eq!(vim.mode(), Mode::Insert);

        send_key(&mut vim, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(vim.mode(), Mode::Normal);

        assert_eq!(
            send_char(&mut vim, 'o'),
            Action::SetToOpenLineBelow { count: 1 }
        );
        assert_eq!(vim.mode(), Mode::Insert);

        send_key(&mut vim, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(vim.mode(), Mode::Normal);

        send_char(&mut vim, '3');
        assert_eq!(
            send_char(&mut vim, 'O'),
            Action::SetToOpenLineAbove { count: 3 }
        );
        assert_eq!(vim.mode(), Mode::Insert);

        send_key(&mut vim, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(vim.mode(), Mode::Normal);

        assert_eq!(
            send_char(&mut vim, 'I'),
            Action::SetToInsertStartOfLineNonSpace
        );
        assert_eq!(vim.mode(), Mode::Insert);

        send_key(&mut vim, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(vim.mode(), Mode::Normal);

        send_char(&mut vim, 'v');
        assert_eq!(vim.mode(), Mode::Visual);
    }

    #[test]
    fn test_insert_mode() {
        let mut vim = VimInput::new();
        send_char(&mut vim, 'i');
        assert_eq!(vim.mode(), Mode::Insert);

        assert_eq!(
            send_char(&mut vim, 'a'),
            Action::InsertText("a".to_string())
        );

        assert_eq!(
            send_key(&mut vim, KeyCode::Enter, KeyModifiers::NONE),
            Action::InsertNewLine { count: 1 }
        );

        send_key(&mut vim, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(vim.mode(), Mode::Normal);
    }

    #[test]
    fn test_count_with_zero() {
        let mut vim = VimInput::new();

        send_char(&mut vim, '1');
        send_char(&mut vim, '0');
        assert_eq!(
            send_char(&mut vim, 'w'),
            Action::MoveToWord {
                count: 10,
                select: false
            }
        );

        assert_eq!(
            send_char(&mut vim, '0'),
            Action::MoveToStartOfLine {
                count: 1,
                select: false
            }
        );
    }

    #[test]
    fn test_invalid_prefix_recovery() {
        let mut vim = VimInput::new();

        // Typing invalid letters 'z' then 'x' prior to 'w' recovers and executes 'w'
        send_char(&mut vim, 'z');
        send_char(&mut vim, 'x');
        assert_eq!(
            send_char(&mut vim, 'w'),
            Action::MoveToWord {
                count: 1,
                select: false
            }
        );
    }
}
