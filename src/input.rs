use crate::actions::Mode::{Command, VisualBlock, VisualLine};
use crate::actions::{Action, Mode};
use crate::document::BufferText;
use crate::document::Document;
use crate::editor::{Editor, EditorBuffer};
use crate::keymap::KeyCombo;
use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEventKind};

pub fn apply_context(action: Action, select: bool, count: u32) -> Action {
    match action {
        Action::MoveUp { .. } => Action::MoveUp { select, count },
        Action::MoveDown { .. } => Action::MoveDown { select, count },
        Action::MoveLeft { .. } => Action::MoveLeft { select, count },
        Action::MoveRight { .. } => Action::MoveRight { select, count },
        Action::MoveToPreviousWord { .. } => Action::MoveToPreviousWord { select, count },
        Action::MoveToNextWord { .. } => Action::MoveToNextWord { select, count },
        Action::MoveToPreviousWordEnd { .. } => Action::MoveToPreviousWordEnd { select, count },
        Action::MoveToNextWordEnd { .. } => Action::MoveToNextWordEnd { select, count },
        Action::MoveToStartOfDocument { .. } => Action::MoveToStartOfDocument { select },
        Action::MoveToEndOfDocument { .. } => Action::MoveToEndOfDocument { select },
        Action::MoveToStartOfLine { .. } => Action::MoveToStartOfLine { select },
        Action::MoveToStartOfLineNonSpace { .. } => Action::MoveToStartOfLineNonSpace { select },
        Action::MoveToEndOfLine { .. } => Action::MoveToEndOfLine { select },
        Action::MoveToLine { line, .. } => Action::MoveToLine { select, line },
        Action::MoveToPreviousParagraph { .. } => Action::MoveToPreviousParagraph { select, count },
        Action::MoveToNextParagraph { .. } => Action::MoveToNextParagraph { select, count },
        Action::MoveToPreviousCharacter { char, .. } => Action::MoveToPreviousCharacter {
            select,
            count,
            char,
        },
        Action::MoveToNextCharacter { char, .. } => Action::MoveToNextCharacter {
            select,
            count,
            char,
        },
        Action::DeleteText { .. } => Action::DeleteText {
            count: count as usize,
        },
        Action::Delete { .. } => Action::Delete { count },
        Action::DeleteCurrentLine { .. } => Action::DeleteCurrentLine { count },
        Action::DeleteMotion { motion, .. } => Action::DeleteMotion {
            count,
            motion: Box::new(apply_context(*motion, true, 1)),
        },
        Action::ChangeMotion { motion, .. } => Action::ChangeMotion {
            count,
            motion: Box::new(apply_context(*motion, true, 1)),
        },
        Action::ChangeCurrentLine { .. } => Action::ChangeCurrentLine { count },
        Action::YankMotion { motion, .. } => Action::YankMotion {
            count,
            motion: Box::new(apply_context(*motion, true, 1)),
        },
        Action::YankCurrentLine { .. } => Action::YankCurrentLine { count },
        Action::Paste { .. } => Action::Paste { count },
        Action::MoveToNextFunction { .. } => Action::MoveToNextFunction { select, count },
        Action::MoveToPreviousFunction { .. } => Action::MoveToPreviousFunction { select, count },
        Action::MoveToNextBlock { .. } => Action::MoveToNextBlock { select, count },
        Action::MoveToPreviousBlock { .. } => Action::MoveToPreviousBlock { select, count },
        Action::MoveToBlockStart { .. } => Action::MoveToBlockStart { select, count },
        Action::MoveToBlockEnd { .. } => Action::MoveToBlockEnd { select, count },
        Action::MoveToNextClass { .. } => Action::MoveToNextClass { select, count },
        Action::MoveToPreviousClass { .. } => Action::MoveToPreviousClass { select, count },
        Action::MoveToNextArgument { .. } => Action::MoveToNextArgument { select, count },
        Action::MoveToPreviousArgument { .. } => Action::MoveToPreviousArgument { select, count },
        Action::Undo { .. } => Action::Undo { count },
        Action::Redo { .. } => Action::Redo { count },
        Action::SetInsertModeMotion { motion } => Action::SetInsertModeMotion {
            motion: Box::new(apply_context(*motion, select, count)),
        },
        other => other,
    }
}

fn resolve_pattern_pending_action(cmd: &str, select: bool, count: u32) -> Option<Action> {
    if cmd.starts_with("df") && cmd.len() == 3 {
        let ch = cmd.chars().nth(2).unwrap();
        Some(Action::DeleteMotion {
            count,
            motion: Box::new(Action::MoveToNextCharacter {
                select: true,
                count: 1,
                char: ch,
            }),
        })
    } else if cmd.starts_with("dF") && cmd.len() == 3 {
        let ch = cmd.chars().nth(2).unwrap();
        Some(Action::DeleteMotion {
            count,
            motion: Box::new(Action::MoveToPreviousCharacter {
                select: true,
                count: 1,
                char: ch,
            }),
        })
    } else if cmd.starts_with("cf") && cmd.len() == 3 {
        let ch = cmd.chars().nth(2).unwrap();
        Some(Action::ChangeMotion {
            count,
            motion: Box::new(Action::MoveToNextCharacter {
                select: true,
                count: 1,
                char: ch,
            }),
        })
    } else if cmd.starts_with("cF") && cmd.len() == 3 {
        let ch = cmd.chars().nth(2).unwrap();
        Some(Action::ChangeMotion {
            count,
            motion: Box::new(Action::MoveToPreviousCharacter {
                select: true,
                count: 1,
                char: ch,
            }),
        })
    } else if cmd.starts_with('f') && cmd.len() == 2 {
        let ch = cmd.chars().nth(1).unwrap();
        Some(Action::MoveToNextCharacter {
            select,
            count,
            char: ch,
        })
    } else if cmd.starts_with('F') && cmd.len() == 2 {
        let ch = cmd.chars().nth(1).unwrap();
        Some(Action::MoveToPreviousCharacter {
            select,
            count,
            char: ch,
        })
    } else {
        None
    }
}

pub enum HandleEvent {
    Redraw,
    RedrawAndSync,
    NoRedraw,
    Exit,
}

pub fn handle_event(editor: &mut Editor, event: Event, visible_rows: i32) -> HandleEvent {
    // Mouse events: currently no-op placeholders
    let mut scroll_up = false;
    let mut scroll_down = false;

    if let Event::Mouse(mouse_event) = &event {
        match mouse_event.kind {
            MouseEventKind::ScrollUp => {
                scroll_up = true;
            }
            MouseEventKind::ScrollDown => {
                scroll_down = true;
            }
            _ => {}
        }
        return HandleEvent::NoRedraw;
    }

    // Bracketed paste
    if let Event::Paste(content) = &event {
        if editor.mode == Mode::Insert {
            editor.apply_active_action(&Action::InsertText(content.clone()));
            return HandleEvent::RedrawAndSync;
        }
        return HandleEvent::NoRedraw;
    }

    if let Event::Key(key_event) = event {
        let mut should_redraw = false;
        let mut should_sync = false;
        let mut current_mode = editor.mode.clone();

        {
            let active_buffer = editor.buffer_manager.active_mut();
            current_mode = active_buffer.doc.current_mode();
        }

        let combo = KeyCombo::from(&key_event);

        // Global actions
        match (key_event.code, key_event.modifiers) {
            (KeyCode::Esc, _) => {
                should_redraw = true;
                let active_buffer = editor.buffer_manager.active_mut();

                let clear_cursors =
                    current_mode != Mode::Normal && active_buffer.doc.has_selection();
                if current_mode == Mode::Normal {
                    active_buffer.doc.clear_selections();
                }

                active_buffer.doc.enter_mode(Mode::Normal);
                current_mode = Mode::Normal;
                if clear_cursors {
                    editor.apply_active_action(&Action::ClearCursors);
                }
            }
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => return HandleEvent::Exit,
            _ => {}
        }

        // Count from pending_cmd prefix
        let count = {
            let mut count_str = String::new();
            let mut parsing_count = true;
            for ch in editor.pending_cmd.chars() {
                if parsing_count && ch.is_ascii_digit() {
                    count_str.push(ch);
                } else {
                    parsing_count = false;
                }
            }
            count_str.parse::<u32>().unwrap_or(1)
        };

        let select = editor.mode == Mode::Visual
            || editor.mode == Mode::VisualLine
            || editor.mode == Mode::VisualBlock;

        // Resolve motions from keymap (applies to normal, visual, and insert/command modes)
        let mut move_action = if let Some(action) = editor.keymap.get_normal_action(&combo) {
            let is_motion = matches!(
                action,
                Action::MoveLeft { .. }
                    | Action::MoveRight { .. }
                    | Action::MoveUp { .. }
                    | Action::MoveDown { .. }
                    | Action::MoveToStartOfLine { .. }
                    | Action::MoveToEndOfLine { .. }
                    | Action::MoveToStartOfLineNonSpace { .. }
                    | Action::MoveToPreviousParagraph { .. }
                    | Action::MoveToNextParagraph { .. }
                    | Action::MoveToPreviousWord { .. }
                    | Action::MoveToNextWord { .. }
                    | Action::MoveToPreviousWordEnd { .. }
                    | Action::MoveToNextWordEnd { .. }
                    | Action::MoveToStartOfDocument { .. }
                    | Action::MoveToEndOfDocument { .. }
                    | Action::MoveToLine { .. }
                    | Action::MoveToPreviousCharacter { .. }
                    | Action::MoveToNextCharacter { .. }
                    | Action::MoveToPreviousMatch { .. }
                    | Action::MoveToNextMatch { .. }
                    | Action::MoveToNextFunction { .. }
                    | Action::MoveToPreviousFunction { .. }
                    | Action::MoveToNextClass { .. }
                    | Action::MoveToPreviousClass { .. }
                    | Action::MoveToNextArgument { .. }
                    | Action::MoveToPreviousArgument { .. }
                    | Action::MoveToNextBlock { .. }
                    | Action::MoveToPreviousBlock { .. }
                    | Action::MoveToBlockStart { .. }
                    | Action::MoveToBlockEnd { .. }
            );
            let is_char_motion = matches!(combo.code, KeyCode::Char(_));
            let is_insert_or_command = editor.mode == Mode::Insert || editor.mode == Mode::Command;

            if is_motion && (!is_insert_or_command || !is_char_motion) {
                let resolved = apply_context(action, select, count);
                match resolved {
                    Action::MoveUp { select, count } if combo.code == KeyCode::PageUp => {
                        Action::MoveUp {
                            select,
                            count: (visible_rows >> 1) as u32 * count,
                        }
                    }
                    Action::MoveDown { select, count } if combo.code == KeyCode::PageDown => {
                        Action::MoveDown {
                            select,
                            count: (visible_rows >> 1) as u32 * count,
                        }
                    }
                    _ => resolved,
                }
            } else {
                Action::NoOp
            }
        } else {
            Action::NoOp
        };

        // Fallback for dead scroll up/down code placeholders
        if move_action == Action::NoOp {
            if scroll_up {
                move_action = Action::MoveUp {
                    select,
                    count: (visible_rows >> 1) as u32 * count,
                };
            } else if scroll_down {
                move_action = Action::MoveDown {
                    select,
                    count: (visible_rows >> 1) as u32 * count,
                };
            }
        }

        // Resolve Normal & Visual Mode actions
        let mut normal_action = Action::NoOp;
        if current_mode == Mode::Normal
            || current_mode == Mode::Visual
            || current_mode == Mode::VisualLine
            || current_mode == Mode::VisualBlock
        {
            let pending_sequence = editor
                .pending_cmd
                .chars()
                .any(|character| !character.is_ascii_digit());

            if combo.code == KeyCode::Esc {
                editor.pending_cmd.clear();
                should_redraw = true;
            } else if !pending_sequence
                && let Some(action) = editor.keymap.get_normal_action(&combo)
            {
                let is_motion = matches!(
                    action,
                    Action::MoveLeft { .. }
                        | Action::MoveRight { .. }
                        | Action::MoveUp { .. }
                        | Action::MoveDown { .. }
                        | Action::MoveToStartOfLine { .. }
                        | Action::MoveToEndOfLine { .. }
                        | Action::MoveToStartOfLineNonSpace { .. }
                        | Action::MoveToPreviousParagraph { .. }
                        | Action::MoveToNextParagraph { .. }
                        | Action::MoveToPreviousWord { .. }
                        | Action::MoveToNextWord { .. }
                        | Action::MoveToPreviousWordEnd { .. }
                        | Action::MoveToNextWordEnd { .. }
                        | Action::MoveToStartOfDocument { .. }
                        | Action::MoveToEndOfDocument { .. }
                        | Action::MoveToLine { .. }
                        | Action::MoveToPreviousCharacter { .. }
                        | Action::MoveToNextCharacter { .. }
                        | Action::MoveToPreviousMatch { .. }
                        | Action::MoveToNextMatch { .. }
                        | Action::MoveToNextFunction { .. }
                        | Action::MoveToPreviousFunction { .. }
                        | Action::MoveToNextClass { .. }
                        | Action::MoveToPreviousClass { .. }
                        | Action::MoveToNextArgument { .. }
                        | Action::MoveToPreviousArgument { .. }
                        | Action::MoveToNextBlock { .. }
                        | Action::MoveToPreviousBlock { .. }
                        | Action::MoveToBlockStart { .. }
                        | Action::MoveToBlockEnd { .. }
                );

                if !is_motion {
                    match action {
                        Action::SetVisualMode => {
                            normal_action = if current_mode == Mode::Visual {
                                Action::SetNormalMode
                            } else {
                                Action::SetVisualMode
                            };
                        }
                        Action::SetVisualLineMode => {
                            normal_action = if current_mode == Mode::VisualLine {
                                Action::SetNormalMode
                            } else {
                                Action::SetVisualLineMode
                            };
                        }
                        Action::SetVisualBlockMode => {
                            normal_action = if current_mode == Mode::VisualBlock {
                                Action::SetNormalMode
                            } else {
                                Action::SetVisualBlockMode
                            };
                        }
                        other_action => {
                            let is_mode_changing = matches!(
                                other_action,
                                Action::SetInsertMode
                                    | Action::SetInsertModeMotion { .. }
                                    | Action::SetCommandMode { .. }
                            );
                            if !(is_mode_changing && current_mode != Mode::Normal) {
                                let resolved = apply_context(other_action, select, count);
                                normal_action = match resolved {
                                    Action::MoveToNextMatch { .. }
                                        if !editor.search_text.is_empty() =>
                                    {
                                        Action::MoveToNextMatch {
                                            search: editor.search_text.clone(),
                                            pattern: editor.pattern,
                                        }
                                    }
                                    Action::MoveToPreviousMatch { .. }
                                        if !editor.search_text.is_empty() =>
                                    {
                                        Action::MoveToPreviousMatch {
                                            search: editor.search_text.clone(),
                                            pattern: editor.pattern,
                                        }
                                    }
                                    _ => resolved,
                                };
                            }
                        }
                    }
                }
            }

            // Fallback for single char / pending commands building
            if normal_action == Action::NoOp {
                if let KeyCode::Char(c) = combo.code {
                    editor.pending_cmd.push(c);
                    let (parsed_count, cmd_without_count) = {
                        let mut count_str = String::new();
                        let mut cmd_str = String::new();
                        let mut parsing_count = true;
                        for ch in editor.pending_cmd.chars() {
                            if parsing_count
                                && ch.is_ascii_digit()
                                && (ch != '0' || !count_str.is_empty())
                            {
                                count_str.push(ch);
                            } else {
                                parsing_count = false;
                                cmd_str.push(ch);
                            }
                        }
                        let count = if count_str.is_empty() {
                            1
                        } else {
                            count_str.parse::<u32>().unwrap_or(1)
                        };
                        (count, cmd_str)
                    };

                    let pending_action = if let Some(action) =
                        editor.keymap.get_pending_action(&cmd_without_count)
                    {
                        Some(action)
                    } else {
                        resolve_pattern_pending_action(&cmd_without_count, select, parsed_count)
                    };

                    if let Some(a) = pending_action {
                        editor.pending_cmd.clear();
                        normal_action = apply_context(a, select, parsed_count);
                    }
                }
            }
        }

        // Resolve Insert & Command Mode Actions
        let insert_action = if let Some(action) = editor.keymap.get_insert_action(&combo) {
            if current_mode == Mode::Insert || current_mode == Mode::Command {
                action
            } else {
                Action::NoOp
            }
        } else if current_mode == Mode::Insert || current_mode == Mode::Command {
            if let KeyCode::Char(c) = combo.code {
                Action::InsertText(c.to_string())
            } else {
                Action::NoOp
            }
        } else {
            Action::NoOp
        };

        if normal_action != Action::NoOp {
            move_action = Action::NoOp;
        }

        //----------------------------
        // handle Action here
        //----------------------------
        match current_mode {
            Mode::Normal => {
                if normal_action != Action::NoOp {
                    let active_buffer = editor.buffer_manager.active_mut();
                    match normal_action {
                        Action::SetInsertMode => {
                            should_redraw = true;
                            active_buffer.doc.enter_mode(Mode::Insert);
                        }
                        Action::SetVisualMode => {
                            should_redraw = true;
                            active_buffer.doc.enter_mode(Mode::Visual);
                        }
                        Action::SetVisualLineMode => {
                            should_redraw = true;
                            active_buffer.doc.enter_mode(VisualLine);
                        }
                        Action::SetVisualBlockMode => {
                            should_redraw = true;
                            active_buffer.doc.enter_mode(VisualBlock);
                        }
                        Action::SetCommandMode { search, pattern } => {
                            should_redraw = true;
                            editor.search = search;
                            editor.pattern = pattern;
                            active_buffer.doc.enter_mode(Command);
                        }
                        Action::NextBuffer => {
                            should_redraw = true;
                            editor.buffer_manager.switch_next();
                        }
                        Action::PreviousBuffer => {
                            should_redraw = true;
                            editor.buffer_manager.switch_prev();
                        }
                        _ => {
                            editor.apply_active_action(&normal_action);
                            editor.pending_cmd.clear();
                        }
                    }
                } else if move_action != Action::NoOp {
                    editor.apply_active_action(&move_action);
                    editor.pending_cmd.clear();
                }
            }
            Mode::Visual | Mode::VisualLine | Mode::VisualBlock => {
                if move_action != Action::NoOp {
                    editor.apply_active_action(&move_action);
                    editor.buffer_manager.active_mut().doc.sync();
                    editor.pending_cmd.clear();
                } else if normal_action != Action::NoOp {
                    editor.apply_active_action(&normal_action);
                    editor.pending_cmd.clear();
                }
            }
            Mode::Insert => {
                editor.pending_cmd.clear();
                if insert_action != Action::NoOp {
                    editor.apply_active_action(&insert_action);
                } else if move_action != Action::NoOp {
                    editor.apply_active_action(&move_action);
                }
            }
            Mode::Command => {
                let mut update_search = false;
                if move_action != Action::NoOp && !editor.search {
                    match move_action {
                        Action::MoveUp { .. } => {
                            if !editor.command_history.is_empty() {
                                let h_idx = editor
                                    .command_history
                                    .len()
                                    .saturating_sub(editor.history_idx + 1);
                                if let Some(h_cmd) = editor.command_history.get(h_idx).cloned() {
                                    editor.cmd = Document::new("").unwrap();
                                    editor.apply_command_action(&Action::InsertText(h_cmd));
                                }
                                if editor.history_idx < editor.command_history.len() {
                                    editor.history_idx += 1;
                                }
                            }
                        }
                        Action::MoveDown { .. } => {
                            if !editor.command_history.is_empty() {
                                editor.cmd = Document::new("").unwrap();
                                if editor.history_idx > 0 {
                                    editor.history_idx -= 1;
                                }
                                let h_idx = editor
                                    .command_history
                                    .len()
                                    .saturating_sub(editor.history_idx);
                                if let Some(h_cmd) = editor.command_history.get(h_idx).cloned() {
                                    editor.apply_command_action(&Action::InsertText(h_cmd));
                                }
                            }
                        }
                        _ => {}
                    }
                    should_redraw = true;
                }

                if move_action != Action::NoOp && editor.search {
                    match move_action {
                        Action::MoveUp { .. } => {
                            if !editor.search_history.is_empty() {
                                let h_idx = editor
                                    .search_history
                                    .len()
                                    .saturating_sub(editor.history_idx + 1);
                                if let Some(h_cmd) = editor.search_history.get(h_idx).cloned() {
                                    editor.cmd = Document::new("").unwrap();
                                    editor.apply_command_action(&Action::InsertText(h_cmd));
                                }
                                if editor.history_idx < editor.search_history.len() {
                                    editor.history_idx += 1;
                                }
                            }
                        }
                        Action::MoveDown { .. } => {
                            if !editor.search_history.is_empty() {
                                editor.cmd = Document::new("").unwrap();
                                if editor.history_idx > 0 {
                                    editor.history_idx -= 1;
                                }
                                let h_idx = editor
                                    .search_history
                                    .len()
                                    .saturating_sub(editor.history_idx);
                                if let Some(h_cmd) = editor.search_history.get(h_idx).cloned() {
                                    editor.apply_command_action(&Action::InsertText(h_cmd));
                                }
                            }
                        }
                        _ => {}
                    }
                    should_redraw = true;
                }

                if let (KeyCode::Enter, _) = (key_event.code, key_event.modifiers) {
                    let command_text = editor.cmd.buffer().row_text(0);

                    // TODO resolve into action
                    if editor.search {
                        if !command_text.is_empty() {
                            editor.search_text = command_text.clone();
                            editor.search_history.push(command_text);

                            editor.apply_active_action(&Action::MoveToNextMatch {
                                search: editor.search_text.clone(),
                                pattern: editor.pattern,
                            });
                        }
                    } else {
                        let command_parts: Vec<&str> =
                            command_text.trim().split_whitespace().collect();

                        if !command_parts.is_empty() {
                            let mut exit = false;
                            let save_command = true;
                            match command_parts[0] {
                                "q" => {
                                    exit = true;
                                }
                                "bn" => {
                                    editor.buffer_manager.switch_next();
                                }
                                "bp" => {
                                    editor.buffer_manager.switch_prev();
                                }
                                "e" if command_parts.len() > 1 => {
                                    let next_id = editor.buffer_manager.buffers.iter().map(|b| b.id).max().unwrap_or(0) + 1;
                                    if let Ok(new_buffer) = EditorBuffer::new(next_id, command_parts[1]) {
                                        editor.buffer_manager.add_buffer(new_buffer);
                                    }
                                }
                                "set" if command_parts.len() > 1 => match command_parts[1] {
                                    "wrap" => editor.wrap = true,
                                    "nowrap" => editor.wrap = false,
                                    "treesitter" | "ts" => {
                                        editor.set_tree_sitter_enabled(true);
                                        should_sync = true;
                                    }
                                    "notreesitter" | "nots" => {
                                        editor.set_tree_sitter_enabled(false);
                                    }
                                    _ if command_parts[1].starts_with("nu") => {
                                        editor.show_line_numbers = true;
                                    }
                                    _ if command_parts[1].starts_with("nonu") => {
                                        editor.show_line_numbers = false;
                                    }
                                    _ => {}
                                },
                                "theme" if command_parts.len() > 1 => {
                                    editor.theme.load_theme(command_parts[1]);
                                    editor.syntax = true;
                                }
                                _ if command_parts[0].starts_with("syn")
                                    && command_parts.len() > 1 =>
                                {
                                    match command_parts[1] {
                                        "on" => {
                                            editor.syntax = true;
                                        }
                                        "off" => editor.syntax = false,
                                        _ => {}
                                    }
                                }
                                cmd if cmd.parse::<u32>().is_ok() => {
                                    let line_number = cmd.parse::<u32>().unwrap();
                                    editor.apply_active_action(&Action::MoveToLine {
                                        select: false,
                                        line: line_number,
                                    });
                                }
                                _ => {}
                            }

                            if save_command {
                                editor.command_history.push(command_text.trim().to_string());
                            }

                            if exit {
                                return HandleEvent::Exit;
                            }
                        }
                    }

                    // Clear command buffer and return to Normal mode
                    editor.cmd = Document::new("").unwrap();
                    {
                        let active_buffer = editor.buffer_manager.active_mut();
                        active_buffer.doc.enter_mode(Mode::Normal);
                    }
                    should_redraw = true;
                } else if insert_action != Action::NoOp {
                    editor.apply_command_action(&insert_action);
                    update_search = true;
                } else if let (KeyCode::Backspace, _) = (key_event.code, key_event.modifiers) {
                    editor.apply_command_action(&Action::Backspace { count: 1 });
                    update_search = true;
                } else if let (KeyCode::Left, _) = (key_event.code, key_event.modifiers) {
                    editor.apply_command_action(&Action::Backspace { count: 1 });
                }

                if current_mode == Mode::Command && update_search && editor.search {
                    editor.search_text = editor.cmd.buffer().row_text(0);
                }
            }
        }

        if normal_action != Action::NoOp
            || move_action != Action::NoOp
            || insert_action != Action::NoOp
        {
            should_redraw = true;
        }
        if normal_action != Action::NoOp || insert_action != Action::NoOp {
            should_sync = true;
        }

        return if should_sync {
            HandleEvent::RedrawAndSync
        } else if should_redraw {
            HandleEvent::Redraw
        } else {
            HandleEvent::NoRedraw
        };
    }

    HandleEvent::NoRedraw
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;
    use text::ToPoint;

    #[test]
    fn pending_sequences_take_priority_over_single_key_actions() {
        let mut editor = Editor::new(Vec::new()).unwrap();

        handle_event(
            &mut editor,
            Event::Key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::empty())),
            20,
        );
        assert_eq!(editor.pending_cmd, "]");

        handle_event(
            &mut editor,
            Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty())),
            20,
        );

        assert!(editor.pending_cmd.is_empty());
        assert_eq!(
            editor.buffer_manager.active().doc.current_mode(),
            Mode::Normal
        );
    }

    #[test]
    fn pending_sequences_take_priority_over_single_key_actions_in_visual_mode() {
        let mut editor = Editor::new(Vec::new()).unwrap();
        editor
            .buffer_manager
            .active_mut()
            .doc
            .enter_mode(Mode::Visual);

        handle_event(
            &mut editor,
            Event::Key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::empty())),
            20,
        );
        assert_eq!(editor.pending_cmd, "]");

        handle_event(
            &mut editor,
            Event::Key(KeyEvent::new(KeyCode::Char('{'), KeyModifiers::empty())),
            20,
        );

        assert!(editor.pending_cmd.is_empty());
    }

    #[test]
    fn test_pageup_pagedown_movement_amount() {
        let mut editor = Editor::new(Vec::new()).unwrap();
        editor.apply_active_action(&Action::InsertText(
            "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10"
                .into(),
        ));

        let start_point = editor
            .buffer_manager
            .active()
            .doc
            .selection()
            .head()
            .to_point(editor.buffer_manager.active().doc.buffer());
        assert_eq!(start_point.row, 9); // cursor starts at line 10 (row 9)

        handle_event(
            &mut editor,
            Event::Key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty())),
            10, // visible_rows
        );

        let end_point = editor
            .buffer_manager
            .active()
            .doc
            .selection()
            .head()
            .to_point(editor.buffer_manager.active().doc.buffer());
        // visible_rows >> 1 = 5. So row should be 9 - 5 = 4.
        assert_eq!(end_point.row, 4);
    }
}
