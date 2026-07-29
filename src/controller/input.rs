use crate::controller;
use crate::controller::actions::{Action, Mode};
use crate::controller::keymap::{InputStateMachine, KeyCombo, Keymap};
use crate::editor::document::BufferText;
use crate::services::clipboard;
use crossterm::event::{Event, KeyCode, KeyEvent, MouseEventKind};

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
