use crate::actions::Mode::{Command, VisualBlock, VisualLine};
use crate::actions::{Action, Mode, SelectInKind};
use crate::document::BufferText;
use crate::document::Document;
use crate::editor::{Editor, EditorBuffer};
use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEventKind};

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

    // event to Action
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
            let active_buffer = editor.buffer_manager.active_mut();
            active_buffer
                .doc
                .apply_action(&Action::InsertText(content.clone()));
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

        // Global actions
        match (key_event.code, key_event.modifiers) {
            (KeyCode::Esc, _) => {
                should_redraw = true;
                let active_buffer = editor.buffer_manager.active_mut();

                if current_mode == Mode::Normal {
                    active_buffer.doc.clear_selections();
                } else {
                    if active_buffer.doc.has_selection() {
                        active_buffer.doc.apply_action(&Action::ClearCursors);
                    }
                }

                active_buffer.doc.enter_mode(Mode::Normal);
                current_mode = Mode::Normal;
            }
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => return HandleEvent::Exit,
            _ => {}
        }

        // Count from pending_cmd prefix
        let (count, _) = {
            let mut count_str = String::new();
            let mut parsing_count = true;
            for ch in editor.pending_cmd.chars() {
                if parsing_count && ch.is_ascii_digit() {
                    count_str.push(ch);
                } else {
                    parsing_count = false;
                }
            }
            let count = count_str.parse::<u32>().unwrap_or(1);
            (count, ())
        };

        let select = editor.mode == Mode::Visual || editor.mode == Mode::VisualLine;

        // Motions (arrow keys et al.)
        let move_action = match (key_event.code, key_event.modifiers) {
            (KeyCode::Left, _) => Action::MoveLeft { select, count },
            (KeyCode::Right, _) => Action::MoveRight { select, count },
            (KeyCode::Up, _) => Action::MoveUp { select, count },
            (KeyCode::Down, _) => Action::MoveDown { select, count },
            _ if scroll_up => Action::MoveUp {
                select,
                count: (visible_rows >> 1) as u32 * count,
            },
            (KeyCode::PageUp, _) => Action::MoveUp {
                select,
                count: (visible_rows >> 1) as u32 * count,
            },
            _ if scroll_down => Action::MoveDown {
                select,
                count: (visible_rows >> 1) as u32 * count,
            },
            (KeyCode::PageDown, _) => Action::MoveDown {
                select,
                count: (visible_rows >> 1) as u32 * count,
            },
            (KeyCode::Home, _) => Action::MoveToStartOfLine { select },
            (KeyCode::End, _) => Action::MoveToEndOfLine { select },
            (KeyCode::Char('0'), _) => Action::MoveToStartOfLine { select },
            (KeyCode::Char('$'), _) => Action::MoveToEndOfLine { select },
            (KeyCode::Char('^'), _) => Action::MoveToStartOfLineNonSpace { select },
            (KeyCode::Char('{'), _) => Action::MoveToPreviousParagraph { select, count },
            (KeyCode::Char('}'), _) => Action::MoveToNextParagraph { select, count },
            _ => Action::NoOp,
        };

        // Normal-mode commands and cmd-building
        let normal_action = match (key_event.code, key_event.modifiers) {
            (KeyCode::Esc, _) => {
                editor.pending_cmd.clear();
                should_redraw = true;
                Action::NoOp
            }
            (KeyCode::Char('i'), _) if editor.mode == Mode::Normal => Action::SetInsertMode,
            (KeyCode::Char('I'), _) if editor.mode == Mode::Normal => Action::SetInsertModeMotion {
                motion: Box::new(Action::MoveToStartOfLine { select }),
            },
            (KeyCode::Char('a'), _) if editor.mode == Mode::Normal => Action::SetInsertModeMotion {
                motion: Box::new(Action::MoveRight { select, count }),
            },
            (KeyCode::Char('A'), _) if editor.mode == Mode::Normal => Action::SetInsertModeMotion {
                motion: Box::new(Action::MoveToEndOfLine { select }),
            },
            (KeyCode::Char('V'), _) if editor.mode == Mode::Normal => Action::SetVisualLineMode,
            (KeyCode::Char('v'), KeyModifiers::CONTROL) if editor.mode == Mode::Normal => {
                Action::SetVisualBlockMode
            }
            (KeyCode::Char('v'), _) if editor.mode == Mode::Normal => Action::SetVisualMode,
            (KeyCode::Char(':'), _) if editor.mode == Mode::Normal => Action::SetCommandMode {
                search: false,
                pattern: false,
            },
            (KeyCode::Char('/'), _) if editor.mode == Mode::Normal => Action::SetCommandMode {
                search: true,
                pattern: false,
            },
            (KeyCode::Char('?'), _) if editor.mode == Mode::Normal => Action::SetCommandMode {
                search: true,
                pattern: true,
            },
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => Action::Redo { count },
            (KeyCode::Char('u'), _) => Action::Undo { count },
            (KeyCode::Char('h'), _) => Action::MoveLeft { select, count },
            (KeyCode::Char('l'), _) => Action::MoveRight { select, count },
            (KeyCode::Char('k'), _) => Action::MoveUp { select, count },
            (KeyCode::Char('j'), _) => Action::MoveDown { select, count },
            (KeyCode::Char('n'), _) if !editor.search_text.is_empty() => Action::MoveToNextMatch {
                search: editor.search_text.clone(),
                pattern: editor.pattern,
            },
            (KeyCode::Char('N'), _) if !editor.search_text.is_empty() => {
                Action::MoveToPreviousMatch {
                    search: editor.search_text.clone(),
                    pattern: editor.pattern,
                }
            }
            (KeyCode::Delete, _) => Action::DeleteText {
                count: count as usize,
            },
            (KeyCode::Backspace, _) => Action::MoveLeft { select, count },
            (KeyCode::Left, KeyModifiers::SHIFT) => Action::MoveToPreviousWord {
                select: false,
                count,
            },
            (KeyCode::Right, KeyModifiers::SHIFT) => Action::MoveToNextWord {
                select: false,
                count,
            },
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => Action::SelectIn {
                kind: SelectInKind::Word,
            },
            (KeyCode::Char(c), _) => {
                editor.pending_cmd.push(c);
                let (count, cmd_without_count) = {
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

                let action = match cmd_without_count.as_str() {
                    "iw" => Some(Action::SelectIn {
                        kind: SelectInKind::Word,
                    }),
                    "aw" => Some(Action::SelectAround {
                        kind: SelectInKind::Word,
                    }),
                    "gg" => Some(Action::MoveToStartOfDocument { select }),
                    "G" => Some(Action::MoveToEndOfDocument { select }),
                    "dd" => Some(Action::DeleteCurrentLine { count }),
                    "dw" => Some(Action::DeleteMotion {
                        count,
                        motion: Box::new(Action::MoveToNextWord {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "db" => Some(Action::DeleteMotion {
                        count,
                        motion: Box::new(Action::MoveToPreviousWord {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "de" => Some(Action::DeleteMotion {
                        count,
                        motion: Box::new(Action::MoveToNextWordEnd {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "dge" => Some(Action::DeleteMotion {
                        count,
                        motion: Box::new(Action::MoveToPreviousWordEnd {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "dj" => Some(Action::DeleteMotion {
                        count,
                        motion: Box::new(Action::MoveDown {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "dk" => Some(Action::DeleteMotion {
                        count,
                        motion: Box::new(Action::MoveUp {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "dh" => Some(Action::DeleteMotion {
                        count,
                        motion: Box::new(Action::MoveLeft {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "dl" => Some(Action::DeleteMotion {
                        count,
                        motion: Box::new(Action::MoveRight {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "d0" => Some(Action::DeleteMotion {
                        count,
                        motion: Box::new(Action::MoveToStartOfLine { select: true }),
                    }),
                    "d$" => Some(Action::DeleteMotion {
                        count,
                        motion: Box::new(Action::MoveToEndOfLine { select: true }),
                    }),
                    "d^" => Some(Action::DeleteMotion {
                        count,
                        motion: Box::new(Action::MoveToStartOfLineNonSpace { select: true }),
                    }),
                    "d{" => Some(Action::DeleteMotion {
                        count,
                        motion: Box::new(Action::MoveToPreviousParagraph {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "d}" => Some(Action::DeleteMotion {
                        count,
                        motion: Box::new(Action::MoveToNextParagraph {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "D" => Some(Action::DeleteMotion {
                        count,
                        motion: Box::new(Action::MoveToEndOfLine { select: true }),
                    }),
                    s if s.starts_with("df") && s.len() == 3 => {
                        let ch = s.chars().nth(2).unwrap();
                        Some(Action::DeleteMotion {
                            count,
                            motion: Box::new(Action::MoveToNextCharacter {
                                select: true,
                                count: 1,
                                char: ch,
                            }),
                        })
                    }
                    s if s.starts_with("dF") && s.len() == 3 => {
                        let ch = s.chars().nth(2).unwrap();
                        Some(Action::DeleteMotion {
                            count,
                            motion: Box::new(Action::MoveToPreviousCharacter {
                                select: true,
                                count: 1,
                                char: ch,
                            }),
                        })
                    }
                    "cc" => Some(Action::ChangeCurrentLine { count }),
                    "cw" => Some(Action::ChangeMotion {
                        count,
                        motion: Box::new(Action::MoveToNextWord {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "cb" => Some(Action::ChangeMotion {
                        count,
                        motion: Box::new(Action::MoveToPreviousWord {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "ce" => Some(Action::ChangeMotion {
                        count,
                        motion: Box::new(Action::MoveToNextWordEnd {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "cge" => Some(Action::ChangeMotion {
                        count,
                        motion: Box::new(Action::MoveToPreviousWordEnd {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "cj" => Some(Action::ChangeMotion {
                        count,
                        motion: Box::new(Action::MoveDown {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "ck" => Some(Action::ChangeMotion {
                        count,
                        motion: Box::new(Action::MoveUp {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "ch" => Some(Action::ChangeMotion {
                        count,
                        motion: Box::new(Action::MoveLeft {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "cl" => Some(Action::ChangeMotion {
                        count,
                        motion: Box::new(Action::MoveRight {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "c0" => Some(Action::ChangeMotion {
                        count,
                        motion: Box::new(Action::MoveToStartOfLine { select: true }),
                    }),
                    "c$" => Some(Action::ChangeMotion {
                        count,
                        motion: Box::new(Action::MoveToEndOfLine { select: true }),
                    }),
                    "c^" => Some(Action::ChangeMotion {
                        count,
                        motion: Box::new(Action::MoveToStartOfLineNonSpace { select: true }),
                    }),
                    "c{" => Some(Action::ChangeMotion {
                        count,
                        motion: Box::new(Action::MoveToPreviousParagraph {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "c}" => Some(Action::ChangeMotion {
                        count,
                        motion: Box::new(Action::MoveToNextParagraph {
                            select: true,
                            count: 1,
                        }),
                    }),
                    "C" => Some(Action::ChangeMotion {
                        count,
                        motion: Box::new(Action::MoveToEndOfLine { select: true }),
                    }),
                    s if s.starts_with("cf") && s.len() == 3 => {
                        let ch = s.chars().nth(2).unwrap();
                        Some(Action::ChangeMotion {
                            count,
                            motion: Box::new(Action::MoveToNextCharacter {
                                select: true,
                                count: 1,
                                char: ch,
                            }),
                        })
                    }
                    s if s.starts_with("cF") && s.len() == 3 => {
                        let ch = s.chars().nth(2).unwrap();
                        Some(Action::ChangeMotion {
                            count,
                            motion: Box::new(Action::MoveToPreviousCharacter {
                                select: true,
                                count: 1,
                                char: ch,
                            }),
                        })
                    }
                    "x" => Some(Action::Delete { count }),
                    "b" => Some(Action::MoveToPreviousWord { select, count }),
                    "w" => Some(Action::MoveToNextWord { select, count }),
                    "e" => Some(Action::MoveToNextWordEnd { select, count }),
                    "ge" => Some(Action::MoveToPreviousWordEnd { select, count }),
                    s if s.starts_with('f') && s.len() == 2 => {
                        let ch = s.chars().nth(1).unwrap();
                        Some(Action::MoveToNextCharacter {
                            select,
                            count,
                            char: ch,
                        })
                    }
                    s if s.starts_with('F') && s.len() == 2 => {
                        let ch = s.chars().nth(1).unwrap();
                        Some(Action::MoveToPreviousCharacter {
                            select,
                            count,
                            char: ch,
                        })
                    }
                    _ => None,
                };

                if let Some(a) = action {
                    editor.pending_cmd.clear();
                    a
                } else {
                    Action::NoOp
                }
            }
            _ => Action::NoOp,
        };

        // Insert & command text input
        let insert_action = match (key_event.code, key_event.modifiers) {
            (KeyCode::Enter, _) if editor.mode == Mode::Insert => Action::InsertNewLine,
            (KeyCode::Tab, _) if editor.mode == Mode::Insert || editor.mode == Mode::Command => {
                Action::InsertTab
            }
            (KeyCode::Delete, _) if editor.mode == Mode::Insert || editor.mode == Mode::Command => {
                Action::Delete { count: 1 }
            }
            (KeyCode::Backspace, _)
                if editor.mode == Mode::Insert || editor.mode == Mode::Command =>
            {
                Action::Backspace
            }
            (KeyCode::Char(c), _)
                if editor.mode == Mode::Insert || editor.mode == Mode::Command =>
            {
                Action::InsertText(c.to_string())
            }
            _ => Action::NoOp,
        };

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
                        _ => {
                            active_buffer.doc.apply_action(&normal_action);
                            editor.pending_cmd.clear();
                        }
                    }
                } else if move_action != Action::NoOp {
                    let active_buffer = editor.buffer_manager.active_mut();
                    active_buffer.doc.apply_action(&move_action);
                    editor.pending_cmd.clear();
                }
            }
            Mode::Visual | Mode::VisualLine | Mode::VisualBlock => {
                if normal_action != Action::NoOp {
                    let active_buffer = editor.buffer_manager.active_mut();
                    active_buffer.doc.apply_action(&normal_action);
                    editor.pending_cmd.clear();
                } else if move_action != Action::NoOp {
                    let active_buffer = editor.buffer_manager.active_mut();
                    active_buffer.doc.apply_action(&move_action);
                    active_buffer.doc.sync();
                    editor.pending_cmd.clear();
                }
            }
            Mode::Insert => {
                editor.pending_cmd.clear();
                if insert_action != Action::NoOp {
                    let active_buffer = editor.buffer_manager.active_mut();
                    active_buffer.doc.apply_action(&insert_action);
                } else if move_action != Action::NoOp {
                    let active_buffer = editor.buffer_manager.active_mut();
                    active_buffer.doc.apply_action(&move_action);
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
                                if let Some(h_cmd) = editor.command_history.get(h_idx) {
                                    editor.cmd = Document::new("").unwrap();
                                    editor.cmd.apply_action(&Action::InsertText(h_cmd.clone()));
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
                                if let Some(h_cmd) = editor.command_history.get(h_idx) {
                                    editor.cmd.apply_action(&Action::InsertText(h_cmd.clone()));
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
                                if let Some(h_cmd) = editor.search_history.get(h_idx) {
                                    editor.cmd = Document::new("").unwrap();
                                    editor.cmd.apply_action(&Action::InsertText(h_cmd.clone()));
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
                                if let Some(h_cmd) = editor.search_history.get(h_idx) {
                                    editor.cmd.apply_action(&Action::InsertText(h_cmd.clone()));
                                }
                            }
                        }
                        _ => {}
                    }
                    should_redraw = true;
                }

                if let (KeyCode::Enter, _) = (key_event.code, key_event.modifiers) {
                    let command_text = editor.cmd.buffer().row_text(0);

                    if editor.search {
                        if !command_text.is_empty() {
                            editor.search_text = command_text.clone();
                            editor.search_history.push(command_text);

                            let active_buffer = editor.buffer_manager.active_mut();
                            active_buffer.doc.apply_action(&Action::MoveToNextMatch {
                                search: editor.search_text.clone(),
                                pattern: editor.pattern,
                            });
                        }
                    } else {
                        let command_parts: Vec<&str> =
                            command_text.trim().split_whitespace().collect();

                        // refactor to generate Action

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
                                    if let Ok(new_buffer) = EditorBuffer::new(command_parts[1]) {
                                        editor.buffer_manager.add_buffer(new_buffer);
                                    }
                                }
                                "set" if command_parts.len() > 1 => match command_parts[1] {
                                    "wrap" => editor.wrap = true,
                                    "nowrap" => editor.wrap = false,
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
                                            editor.buffer_manager.active_mut().dirty_hl = true;
                                        }
                                        "off" => editor.syntax = false,
                                        _ => {}
                                    }
                                }
                                cmd if cmd.parse::<u32>().is_ok() => {
                                    let line_number = cmd.parse::<u32>().unwrap();
                                    let active_buffer = editor.buffer_manager.active_mut();
                                    active_buffer.doc.apply_action(&Action::MoveToLine {
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
                    editor.cmd.apply_action(&insert_action);
                    update_search = true;
                } else if let (KeyCode::Backspace, _) = (key_event.code, key_event.modifiers) {
                    editor.cmd.apply_action(&Action::Backspace);
                    update_search = true;
                } else if let (KeyCode::Left, _) = (key_event.code, key_event.modifiers) {
                    editor.cmd.apply_action(&Action::Backspace);
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
