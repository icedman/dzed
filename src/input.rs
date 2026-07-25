use crate::actions::{Action, Mode};
use crate::keymap::{KeyCombo, KeyComboSequence, Keymap};
use crossterm::event::{Event, KeyEvent, MouseEventKind};
use std::collections::HashMap;

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
                match action {
                    Action::Redo { .. } => {
                        return HandleEvent::Exit;
                    }
                    _ => {}
                }

                editor.apply_active_action(&action);
                // After applying action, mode might have changed again (e.g. Change action sets Insert mode)
                editor.input.mode = editor.buffer_manager.active().doc.current_mode();
                editor.buffer_manager.active_mut().doc.sync();
                return HandleEvent::RedrawAndSync;
            }

            // Handle Esc specifically if it didn't produce an action but should clear state
            if key_event.code == crossterm::event::KeyCode::Esc {
                editor.input.clear();
                return HandleEvent::Redraw;
            }

            // If sequence is not empty, we might be in the middle of a command, redraw to show it
            if !editor.input.sequence.is_empty() {
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

// pub fn truncate_chars(s: &mut String, n: usize) {
//     let char_count = s.chars().count();
//     let new_len = char_count.saturating_sub(n);
//     if new_len == 0 {
//         s.clear();
//     } else {
//         if let Some((idx, _)) = s.char_indices().nth(new_len) {
//             s.truncate(idx);
//         }
//     }
// }

// pub fn resolve_count(seq: &mut String) -> u32 {
//     let mut digits = String::new();
//     while let Some(c) = seq.chars().last() {
//         if c.is_ascii_digit() {
//             digits.push(seq.pop().unwrap());
//         } else {
//             break;
//         }
//     }
//     if digits.is_empty() {
//         return 1;
//     }
//     digits
//         .chars()
//         .rev()
//         .collect::<String>()
//         .parse::<u32>()
//         .unwrap_or(1)
// }

// pub fn resolve_action(seq: &mut String, map: &HashMap<String, Action>) -> Action {
//     if let Some(lc) = seq.chars().last() {
//         if lc.is_ascii_digit() {
//             if peek_count(seq) > 0 {
//                 return Action::NoOp;
//             }
//         }

//         let last_char = lc.to_string();
//         let mut matched: Option<(String, Action, bool, String)> = None;

//         for (key, action) in map {
//             let mut current_mk = key.as_str();
//             let rk;
//             let current_with_char;

//             if key.ends_with("{c}") {
//                 rk = key.replace("{c}", &last_char);
//                 current_mk = rk.as_str();
//                 current_with_char = true;
//             } else {
//                 current_with_char = false;
//             }

//             if seq.ends_with(current_mk) {
//                 if matched.is_none()
//                     || current_mk.chars().count() > matched.as_ref().unwrap().0.chars().count()
//                 {
//                     matched = Some((
//                         current_mk.to_string(),
//                         action.clone(),
//                         current_with_char,
//                         key.clone(),
//                     ));
//                 }
//             }
//         }

//         if let Some((mk, action, with_char, _key)) = matched {
//             if with_char {
//                 truncate_chars(seq, mk.chars().count());
//                 let count = resolve_count(seq);
//                 return action.with_char(lc, count);
//             }
//             truncate_chars(seq, mk.chars().count());
//             let count = resolve_count(seq);
//             return action.with_count(count);
//         }
//     }

//     Action::NoOp
// }

pub fn peek_action(seq: &KeyComboSequence, map: &HashMap<KeyComboSequence, Action>) -> Action {
    let mut s = seq.clone();
    resolve_action(&mut s, map)
}

pub fn peek_count(seq: &KeyComboSequence) -> u32 {
    let mut s = seq.clone();
    resolve_count(&mut s)
}

pub fn resolve_count(seq: &mut KeyComboSequence) -> u32 {
    return seq.pop_trailing_digits();
}

pub fn resolve_action(seq: &mut KeyComboSequence, map: &HashMap<KeyComboSequence, Action>) -> Action {
    let mut matched: Option<(KeyComboSequence, Action)> = None;
    for (key, action) in map {
        if seq.ends_with(key) {
            matched = Some((key.clone(), action.clone()));
            break;
        }
    }

    if let Some((key, action)) = matched {
        seq.truncate_items(key.len());
        let count = resolve_count(seq);
        return action.with_count(count);
    }

    Action::NoOp
}

pub fn resolve_op_motion_action(motion: Action, action: Action) -> Action {
    let count = action.count();
    match action {
        Action::Delete { .. } => Action::DeleteMotion {
            count,
            motion: Box::new(motion),
        },
        Action::Change { .. } => Action::ChangeMotion {
            count,
            motion: Box::new(motion),
        },
        Action::Yank { .. } => Action::YankMotion {
            count,
            motion: Box::new(motion),
        },
        _ => Action::NoOp,
    }
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
    pub sequence: KeyComboSequence,
    pub resolved_motion: Action,
    pub resolved_op: Action,
    pub resolved_action: Action,
    pub keymap: Keymap,
}

impl VimInput {
    pub fn new() -> Self {
        Self {
            mode: Mode::Normal,
            sequence: KeyComboSequence::new(), // "".to_string(),
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
        self.sequence.clear();
    }

    pub fn handle_sequence(&mut self, sequence: &str) -> Action {
        // self.sequence.clear();
        // self.handle_input(sequence)
        Action::NoOp
    }

    pub fn handle_event(&mut self, key_event: &KeyEvent) -> Action {
        let combo = KeyCombo::from(key_event);
        self.sequence.push(combo.clone());
        return self.handle_input(combo.to_string().as_str());
    }

    pub fn handle_input(&mut self, sequence: &str) -> Action {
        // self.sequence.push_str(sequence);
        self.clear_resolved();

        // insert mode
        let insert_action = if self.mode == Mode::Insert || self.mode == Mode::Command {
            resolve_action(&mut self.sequence, &self.keymap.insert_actions)
        } else {
            Action::NoOp
        };
        if insert_action != Action::NoOp {
            self.resolved_action = insert_action.clone();
            return self.resolved_action.clone();
        }

        // visual mode
        let visual_action = if self.mode.is_visual() {
            resolve_action(&mut self.sequence, &self.keymap.visual_actions)
        } else {
            Action::NoOp
        };
        if visual_action != Action::NoOp {
            self.resolved_action = visual_action.clone();
            return self.resolved_action.clone();
        }

        // normal mode
        // 1. Try to resolve a full motion first (possibly with an operator)
        if self.mode == Mode::Normal || self.mode.is_visual() {
            let mut seq_for_motion = self.sequence.clone();
            let mut motion_action =
                resolve_action(&mut seq_for_motion, &self.keymap.motion_actions);

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

                // Check if there's an operator before the motion
                let op_action = resolve_action(&mut seq_for_motion, &self.keymap.op_actions);

                if op_action != Action::NoOp {
                    self.resolved_op = op_action.clone();
                    self.resolved_action = resolve_op_motion_action(
                        self.resolved_motion.clone(),
                        self.resolved_op.clone(),
                    );
                } else {
                    // Just a motion
                    self.resolved_action = motion_action.clone();
                }

                self.sequence.clear();
            } else if self.resolved_action == Action::NoOp {
                // 2. Try to resolve a normal action (like 'dd' or 'x')
                let mut seq_for_normal = self.sequence.clone();
                let normal_action =
                    resolve_action(&mut seq_for_normal, &self.keymap.normal_actions);
                if normal_action != Action::NoOp {
                    self.resolved_op = normal_action.clone();
                    self.resolved_action = normal_action.clone();
                    self.sequence.clear();
                } else {
                    // 3. Try to resolve a mode change
                    let mut seq_for_mode = self.sequence.clone();
                    let mode_action = resolve_action(&mut seq_for_mode, &self.keymap.mode_actions);
                    if mode_action != Action::NoOp {
                        self.resolved_action = mode_action.clone();
                        self.sequence.clear();
                    } else {
                        // 4. If nothing resolved yet, peek for an operator to update UI
                        let op_action = peek_action(&self.sequence, &self.keymap.op_actions);
                        if op_action != Action::NoOp {
                            self.resolved_op = op_action;
                        }
                    }
                }
            }
        }

        return self.resolved_action.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;

    #[test]
    fn test_resolve_count() {
        let mut s = "123".to_string();
        assert_eq!(resolve_count(&mut s), 123);
        assert_eq!(s, "");

        let mut s = "abc123".to_string();
        assert_eq!(resolve_count(&mut s), 123);
        assert_eq!(s, "abc");

        let mut s = "abc".to_string();
        assert_eq!(resolve_count(&mut s), 1);
        assert_eq!(s, "abc");
    }

    #[test]
    fn test_simple_motions() {
        let mut vim = VimInput::new();

        // Test 'j'
        assert_eq!(
            vim.handle_input("j"),
            Action::MoveDown {
                count: 1,
                select: false
            }
        );

        // Test '5k'
        vim.handle_input("5");
        assert_eq!(
            vim.handle_input("k"),
            Action::MoveUp {
                count: 5,
                select: false
            }
        );

        // Test 'gg'
        vim.handle_input("g");
        assert_eq!(
            vim.handle_input("g"),
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
        vim.handle_input("d");
        let action = vim.handle_input("w");
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
        vim.handle_input("2");
        vim.handle_input("d");
        vim.handle_input("3");
        let action = vim.handle_input("w");
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

        // Test 'd3w'
        vim.handle_input("d");
        vim.handle_input("3");
        let action = vim.handle_input("w");
        if let Action::DeleteMotion { count, motion } = action {
            assert_eq!(count, 1);
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
        vim.handle_input("d");
        assert_eq!(vim.handle_input("d"), Action::DeleteLine { count: 1 });

        // Test '5yy'
        vim.handle_input("5");
        vim.handle_input("y");
        assert_eq!(vim.handle_input("y"), Action::YankLine { count: 5 });
    }

    #[test]
    fn test_char_motions() {
        let mut vim = VimInput::new();

        // Test 'fx'
        vim.handle_input("f");
        assert_eq!(
            vim.handle_input("x"),
            Action::MoveToNextCharacter {
                count: 1,
                select: false,
                ch: 'x'
            }
        );

        // Test '3Fy'
        vim.handle_input("3");
        vim.handle_input("F");
        assert_eq!(
            vim.handle_input("y"),
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
        vim.handle_input("i");
        assert_eq!(vim.mode, Mode::Insert);

        vim.handle_input("Esc");
        assert_eq!(vim.mode, Mode::Normal);

        vim.handle_input("v");
        assert_eq!(vim.mode, Mode::Visual);
    }

    #[test]
    fn test_insert_mode() {
        let mut vim = VimInput::new();
        vim.handle_input("i");
        assert_eq!(vim.mode, Mode::Insert);

        // Test inserting a character
        assert_eq!(vim.handle_input("a"), Action::InsertText("a".to_string()));

        // Test Enter in insert mode
        assert_eq!(
            vim.handle_input("Enter"),
            Action::InsertNewLine { count: 1 }
        );

        // Test escaping back to normal mode
        vim.handle_input("Esc");
        assert_eq!(vim.mode, Mode::Normal);
    }

    #[test]
    fn test_partial_operator() {
        let mut vim = VimInput::new();
        vim.handle_input("d");
        assert_eq!(vim.resolved_op, Action::Delete { count: 1 });

        // Test 'd3' - operator should still be resolved even with count
        vim.handle_input("3");
        // This currently fails because 'd' is not at the end of "d3"
        assert_eq!(vim.resolved_op, Action::Delete { count: 1 });
    }
}
