use crate::actions::{Action, Mode};
use crate::keymap::{
    peek_action, resolve_action, resolve_op_motion_action, KeyBuffer, KeyCombo, Keymap,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind};

pub enum HandleEvent {
    Redraw,
    RedrawAndSync,
    NoRedraw,
    Exit,
}

pub fn handle_event(
    editor: &mut crate::editor::Editor,
    event: Event,
    _visible_rows: i32,
) -> HandleEvent {
    match event {
        Event::Key(key_event) => {
            let action = editor.input.handle_event(&key_event);

            // Sync mode from VimInput to Document
            let active_buffer = editor.buffer_manager.active_mut();
            if active_buffer.doc.current_mode() != editor.input.mode {
                active_buffer.doc.enter_mode(editor.input.mode);
            }

            if action != Action::NoOp {
                if matches!(action, Action::Redo { .. }) {
                    return HandleEvent::Exit;
                }

                editor.apply_active_action(&action);
                // After applying action, mode might have changed again (e.g. Change action sets Insert mode)
                editor.input.mode = editor.buffer_manager.active().doc.current_mode();
                editor.buffer_manager.active_mut().doc.sync();
                return HandleEvent::RedrawAndSync;
            }

            // Handle Esc specifically if it didn't produce an action but should clear state
            if key_event.code == KeyCode::Esc {
                editor.input.clear();
                return HandleEvent::Redraw;
            }

            // If key buffer is not empty, we are in the middle of a command/sequence; redraw to update UI
            if !editor.input.buffer.is_empty() {
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
    pub mode: Mode,
    pub buffer: KeyBuffer,
    pub resolved_motion: Action,
    pub resolved_op: Action,
    pub resolved_action: Action,
    pub keymap: Keymap,
}

impl VimInput {
    pub fn new() -> Self {
        Self {
            mode: Mode::Normal,
            buffer: KeyBuffer::new(),
            resolved_motion: Action::NoOp,
            resolved_op: Action::NoOp,
            resolved_action: Action::NoOp,
            keymap: Keymap::new(),
        }
    }

    pub fn clear_resolved(&mut self) {
        self.resolved_motion = Action::NoOp;
        self.resolved_op = Action::NoOp;
        self.resolved_action = Action::NoOp;
    }

    pub fn clear(&mut self) {
        self.clear_resolved();
        self.buffer.clear();
    }

    pub fn handle_event(&mut self, key_event: &KeyEvent) -> Action {
        // Filter out KeyRelease events from crossterm to avoid duplicate buffer entries
        if key_event.kind == crossterm::event::KeyEventKind::Release {
            return Action::NoOp;
        }

        let combo = KeyCombo::from(key_event);
        self.buffer.push(combo);
        self.process_buffer()
    }

    pub fn process_buffer(&mut self) -> Action {
        self.clear_resolved();

        if self.buffer.is_empty() {
            return Action::NoOp;
        }

        // --- 1. Insert & Command Modes ---
        if self.mode == Mode::Insert || self.mode == Mode::Command {
            let insert_action = resolve_action(&mut self.buffer, &self.keymap.insert_actions);
            if insert_action != Action::NoOp {
                if insert_action == Action::Clear {
                    self.mode = Mode::Normal;
                }
                self.resolved_action = insert_action;
                self.buffer.clear();
                return self.resolved_action.clone();
            }

            // Fallback for typing plain text characters in insert/command mode
            if let Some(combo) = self.buffer.items.last().cloned() {
                if combo.modifiers.is_empty() || combo.modifiers == KeyModifiers::SHIFT {
                    if let KeyCode::Char(c) = combo.code {
                        self.buffer.clear();
                        self.resolved_action = Action::InsertText(c.to_string());
                        return self.resolved_action.clone();
                    }
                }
            }
            return Action::NoOp;
        }

        // --- 2. Visual Mode Overrides ---
        if self.mode.is_visual() {
            let visual_action = resolve_action(&mut self.buffer, &self.keymap.visual_actions);
            if visual_action != Action::NoOp {
                if visual_action == Action::Clear {
                    self.mode = Mode::Normal;
                }
                self.resolved_action = visual_action;
                self.buffer.clear();
                return self.resolved_action.clone();
            }
        }

        // --- 3. Normal & Visual Modes ---
        if self.mode == Mode::Normal || self.mode.is_visual() {
            // A. Try Motions first (e.g., 'w', 'j', 'gg', '2d3w')
            let mut work_buf = self.buffer.clone();
            let mut motion_action = resolve_action(&mut work_buf, &self.keymap.motion_actions);

            if motion_action == Action::NoOp && self.mode.is_visual() {
                motion_action = Action::StandBy {
                    count: 0,
                    select: true,
                };
            }

            if motion_action != Action::NoOp {
                if self.mode.is_visual() {
                    motion_action = motion_action.with_select(true);
                }

                self.resolved_motion = motion_action.clone();

                // Check if an operator was typed before the motion (e.g., 'd' in 'dw')
                let op_action = resolve_action(&mut work_buf, &self.keymap.op_actions);

                if op_action != Action::NoOp {
                    self.resolved_op = op_action.clone();
                    self.resolved_action = resolve_op_motion_action(
                        self.resolved_motion.clone(),
                        self.resolved_op.clone(),
                    );
                } else {
                    self.resolved_action = motion_action;
                }

                self.buffer.clear();
                return self.resolved_action.clone();
            }

            // B. Try Normal Mode line/char commands (e.g., 'dd', 'x', 'u')
            let mut work_buf_normal = self.buffer.clone();
            let normal_action = resolve_action(&mut work_buf_normal, &self.keymap.normal_actions);
            if normal_action != Action::NoOp {
                self.resolved_op = normal_action.clone();
                self.resolved_action = normal_action;
                self.buffer.clear();
                return self.resolved_action.clone();
            }

            // C. Try Mode change bindings (e.g., 'i', 'v', ':')
            let mut work_buf_mode = self.buffer.clone();
            let mode_action = resolve_action(&mut work_buf_mode, &self.keymap.mode_actions);
            if mode_action != Action::NoOp {
                match mode_action {
                    Action::SetToInsert => self.mode = Mode::Insert,
                    Action::SetToVisual => self.mode = Mode::Visual,
                    Action::SetToCommand => self.mode = Mode::Command,
                    _ => {}
                }
                self.resolved_action = mode_action;
                self.buffer.clear();
                return self.resolved_action.clone();
            }

            // D. Partial command state (e.g. user typed 'd' or '2d3'), update operator UI indicator
            let mut peek_buf = self.buffer.clone();
            let _ = peek_buf.pop_trailing_digits();
            let op_action = peek_action(&peek_buf, &self.keymap.op_actions);
            if op_action != Action::NoOp {
                self.resolved_op = op_action;
            }
        }

        self.resolved_action.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        // Test 'j'
        assert_eq!(
            send_char(&mut vim, 'j'),
            Action::MoveDown {
                count: 1,
                select: false
            }
        );

        // Test '5k'
        send_char(&mut vim, '5');
        assert_eq!(
            send_char(&mut vim, 'k'),
            Action::MoveUp {
                count: 5,
                select: false
            }
        );

        // Test 'gg'
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

        // Test 'dw'
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

        // Test '2d3w'
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

        // Test 'dd'
        send_char(&mut vim, 'd');
        assert_eq!(send_char(&mut vim, 'd'), Action::DeleteLine { count: 1 });

        // Test '5yy'
        send_char(&mut vim, '5');
        send_char(&mut vim, 'y');
        assert_eq!(send_char(&mut vim, 'y'), Action::YankLine { count: 5 });
    }

    #[test]
    fn test_char_motions() {
        let mut vim = VimInput::new();

        // Test 'fx'
        send_char(&mut vim, 'f');
        assert_eq!(
            send_char(&mut vim, 'x'),
            Action::MoveToNextCharacter {
                count: 1,
                select: false,
                ch: 'x'
            }
        );

        // Test '3Fy'
        send_char(&mut vim, '3');
        send_char(&mut vim, 'F');
        assert_eq!(
            send_char(&mut vim, 'y'),
            Action::MoveToPreviousCharacter {
                count: 3,
                select: false,
                ch: 'y'
            }
        );
    }

    #[test]
    fn test_mode_changes() {
        let mut vim = VimInput::new();

        assert_eq!(vim.mode, Mode::Normal);
        send_char(&mut vim, 'i');
        assert_eq!(vim.mode, Mode::Insert);

        send_key(&mut vim, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(vim.mode, Mode::Normal);

        send_char(&mut vim, 'v');
        assert_eq!(vim.mode, Mode::Visual);
    }

    #[test]
    fn test_insert_mode() {
        let mut vim = VimInput::new();
        send_char(&mut vim, 'i');
        assert_eq!(vim.mode, Mode::Insert);

        // Test inserting a character
        assert_eq!(send_char(&mut vim, 'a'), Action::InsertText("a".to_string()));

        // Test Enter in insert mode
        assert_eq!(
            send_key(&mut vim, KeyCode::Enter, KeyModifiers::NONE),
            Action::InsertNewLine { count: 1 }
        );

        // Test escaping back to normal mode
        send_key(&mut vim, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(vim.mode, Mode::Normal);
    }

    #[test]
    fn test_partial_operator() {
        let mut vim = VimInput::new();
        send_char(&mut vim, 'd');
        assert_eq!(vim.resolved_op, Action::Delete { count: 1 });

        // Test 'd3' - operator should still be resolved even with trailing count digits
        send_char(&mut vim, '3');
        assert_eq!(vim.resolved_op, Action::Delete { count: 1 });
    }
}