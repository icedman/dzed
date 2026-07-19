mod actions;
mod display;
mod document;
mod highlight;
mod selections;

use std::{
    cmp::Ordering,
    io::{Write, stdout},
    ops::Range,
    time::{Duration, Instant},
};

use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{Clear, ClearType},
};

use text::ToPoint;

use actions::{Action, Mode};
use display::display_map::DisplayMap;
use document::{BufferText, Document};
use highlight::Highlights;

pub trait ToCrossTerm {
    fn rgb(&self) -> crossterm::style::Color;
}

pub trait ColorAdjust {
    fn lighten(&self, amount: u8) -> syntect::highlighting::Color;
    fn darken(&self, amount: u8) -> syntect::highlighting::Color;
}

impl ToCrossTerm for syntect::highlighting::Color {
    fn rgb(&self) -> crossterm::style::Color {
        return crossterm::style::Color::Rgb {
            r: self.r,
            g: self.g,
            b: self.b,
        };
    }
}

impl ColorAdjust for syntect::highlighting::Color {
    fn lighten(&self, amount: u8) -> syntect::highlighting::Color {
        syntect::highlighting::Color {
            r: self.r.saturating_add(amount),
            g: self.g.saturating_add(amount),
            b: self.b.saturating_add(amount),
            a: self.a,
        }
    }

    fn darken(&self, amount: u8) -> syntect::highlighting::Color {
        syntect::highlighting::Color {
            r: self.r.saturating_sub(amount),
            g: self.g.saturating_sub(amount),
            b: self.b.saturating_sub(amount),
            a: self.a,
        }
    }
}

fn fill_to_eol(count: usize) {
    for _ in 0..count {
        print!(" ");
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        return Ok(());
    }

    let file_path = &args[1];
    let mut doc = match Document::new(file_path) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("Failed to open document: {}", err);
            return Ok(());
        }
    };

    let mut cmd = match Document::new("") {
        Ok(doc) => doc,
        Err(_err) => {
            return Ok(());
        }
    };
    let mut pending_cmd = "".to_string();
    let mut mode = Mode::Normal;

    let mut stdout = stdout();
    crossterm::terminal::enable_raw_mode().unwrap();
    execute!(
        stdout,
        crossterm::event::EnableBracketedPaste,
        Clear(ClearType::All),
        MoveTo(0, 0)
    )
    .unwrap();

    let mut hl = Highlights::new(file_path);
    let mut display_map = DisplayMap::new(doc.buffer().snapshot().clone(), None);
    let mut scroll_x: u32 = 0;
    let mut scroll_y: u32 = 0;

    let tab_size = 4;

    // Prepare default background pair
    let (clr_fg, clr_bg, clr_caret, _clr_current_line, clr_select, clr_gutter) = {
        let settings = hl.theme_settings();
        let fg = settings.foreground.unwrap();
        let bg = settings.background.unwrap();
        (
            fg.rgb(),
            bg.darken(10).rgb(),
            settings.caret.unwrap_or(fg).rgb(),
            settings.line_highlight.unwrap_or(bg.darken(10)).rgb(),
            settings.selection.unwrap_or(bg.darken(10)).rgb(),
            settings.gutter.unwrap_or(bg).rgb(),
        )
    };

    execute!(stdout, crossterm::cursor::Hide).unwrap();

    let _paste_buffer = String::new();
    let _last_char_time = Instant::now();
    let _paste_timeout = Duration::from_millis(5); // threshold to separate pastes from normal typing

    let mut dirty_hl = true;
    let mut should_redraw = false;
    let mut prev_screen_rows = 0;
    let mut prev_screen_cols = 0;

    loop {
        // get screen dimensions
        let (screen_cols, screen_rows) = {
            let (cols, rows) = crossterm::terminal::size().unwrap();
            (cols as i32, rows as i32)
        };
        // dimensions has changed
        if prev_screen_cols != screen_cols && prev_screen_rows != screen_rows {
            should_redraw = true;
        }
        prev_screen_rows = screen_rows;
        prev_screen_cols = screen_cols;

        // update display map
        display_map.set_wrap_width(Some(screen_cols as u32));
        display_map.sync(doc.buffer().snapshot().clone());
        let display_snapshot = display_map.snapshot();

        // get cursor information
        let cursor = doc.selection();
        let cursor_head = cursor.head();
        let cursor_tail = cursor.tail();
        let _cursor_range = if cursor_head.cmp(&cursor_tail, &doc.buffer()) == Ordering::Less {
            Range {
                start: cursor_head,
                end: cursor_tail,
            }
        } else {
            Range {
                end: cursor_head,
                start: cursor_tail,
            }
        };
        let cursor_point = cursor_head.to_point(&doc.buffer());
        let display_cursor = display_snapshot.point_to_display_point(cursor_point);
        let cursor_row = display_cursor.row() as i32;
        let cursor_col = display_cursor.column() as i32;
        let visible_rows = screen_rows - 1;
        let visible_cols = screen_cols;

        // scroll based on cursor position
        let mut cursor_screen_row = cursor_row - scroll_y as i32;
        while cursor_screen_row >= visible_rows {
            scroll_y += 1;
            cursor_screen_row = cursor_row - scroll_y as i32;
        }
        while cursor_screen_row < 0 && scroll_y > 0 {
            scroll_y -= 1;
            cursor_screen_row = cursor_row - scroll_y as i32;
        }
        let mut cursor_screen_col = cursor_col - scroll_x as i32;
        while cursor_screen_col >= visible_cols {
            scroll_x += 1;
            cursor_screen_col = cursor_col - scroll_x as i32;
        }
        while cursor_screen_col < 0 && scroll_x > 0 {
            scroll_x -= 1;
            cursor_screen_col = cursor_col - scroll_x as i32;
        }

        // bar
        // print!("\x1b[5 q");

        //------------------
        // render
        //------------------
        if should_redraw {
            should_redraw = false;
            execute!(stdout, crossterm::cursor::Hide).unwrap();

            let buffer = doc.buffer();
            let total_rows = display_snapshot.row_count();
            let end_line = (scroll_y + visible_rows as u32).min(total_rows);

            if dirty_hl {
                let start_buffer_row = display_snapshot.buffer_row_for_display_row(scroll_y);
                let end_buffer_row =
                    display_snapshot.buffer_row_for_display_row(end_line.saturating_sub(1));

                hl.highlight_lines(
                    doc.buffer(),
                    start_buffer_row as usize,
                    (end_buffer_row - start_buffer_row + 1) as usize,
                );
            }
            dirty_hl = true;

            let mut screen_row = 0;
            for row in scroll_y..end_line {
                execute!(stdout, MoveTo(0, screen_row)).unwrap();
                let text = display_snapshot.line_text(row) + " ";
                let buffer_row = display_snapshot.buffer_row_for_display_row(row);
                let buffer_range = display_snapshot.buffer_range_for_display_row(row);
                let start_col = buffer_range.start.column;

                let ranges;
                if let Some(style_cache) = hl.render_line(buffer_row as usize) {
                    ranges = &style_cache.styles;
                } else {
                    execute!(stdout, crossterm::style::SetBackgroundColor(clr_bg)).unwrap();
                    fill_to_eol(screen_cols as usize);
                    screen_row += 1;
                    continue;
                }

                // style range
                let mut range_iter = ranges.iter();
                let mut current_range = range_iter.next();

                // Skip ranges that end before our start_col
                while let Some((_, _s, e)) = current_range {
                    if *e <= start_col {
                        current_range = range_iter.next();
                    } else {
                        break;
                    }
                }

                let mut range_remaining =
                    current_range.map_or(
                        0,
                        |(_, s, e)| {
                            if *s < start_col { e - start_col } else { e - s }
                        },
                    );
                let mut current_style = current_range.map(|(style, _, _)| style);

                let mut x_scroll = scroll_x;
                let mut cols_remaining = screen_cols;

                for (column, ch) in text.chars().enumerate() {
                    let rc = start_col + column as u32;

                    if range_remaining == 0 {
                        current_range = range_iter.next();
                        range_remaining = current_range.map_or(0, |(_, s, e)| e - s);
                        current_style = current_range.map(|(style, _, _)| style);
                    }

                    let mut fg = clr_fg.clone();
                    let mut bg = clr_bg.clone();

                    if let Some(style) = current_style {
                        fg = style.foreground.rgb();
                        bg = style.background.darken(10).rgb();
                    }

                    let (selected, mut selected_line, at_cursor) =
                        doc.selections().is_selected(buffer_row, rc, &buffer);
                    if selected && (mode == Mode::Visual || mode == Mode::Visual_Line) {
                        bg = clr_select;
                    }
                    selected_line = selected_line && mode == Mode::Visual_Line;
                    if selected_line {
                        bg = clr_select;
                    }
                    if at_cursor && mode != Mode::Insert && mode != Mode::Command {
                        fg = clr_bg;
                        bg = clr_caret;
                    }

                    execute!(stdout, crossterm::style::SetForegroundColor(fg)).unwrap();
                    execute!(stdout, crossterm::style::SetBackgroundColor(bg)).unwrap();

                    if x_scroll > 0 {
                        x_scroll = x_scroll.saturating_sub(1);
                    } else {
                        match ch {
                            '\t' => {
                                for _i in 0..tab_size {
                                    print!(" ");
                                    if at_cursor && mode != Mode::Insert && mode != Mode::Command {
                                        execute!(
                                            stdout,
                                            crossterm::style::SetBackgroundColor(clr_bg)
                                        )
                                        .unwrap();
                                    }
                                    cols_remaining = cols_remaining.saturating_sub(1);
                                }
                            }
                            _ => {
                                print!("{}", ch);
                                cols_remaining = cols_remaining.saturating_sub(1);
                            }
                        }
                    }

                    range_remaining = range_remaining.saturating_sub(1);

                    if cols_remaining <= 0 {
                        break;
                    }
                }

                execute!(stdout, crossterm::style::SetBackgroundColor(clr_bg)).unwrap();

                // fill_to_eol((screen_cols - text.chars().count() as i32).max(0) as usize);
                fill_to_eol(cols_remaining.max(0) as usize);

                screen_row += 1;
                if screen_row + 1 > screen_rows as u16 {
                    break;
                }
            }

            // statusbar
            {
                execute!(stdout, crossterm::style::SetForegroundColor(clr_fg)).unwrap();
                execute!(stdout, crossterm::style::SetBackgroundColor(clr_gutter)).unwrap();
                execute!(stdout, MoveTo(0, screen_rows as u16)).unwrap();
                fill_to_eol(screen_cols as usize);
                execute!(stdout, MoveTo(0, screen_rows as u16)).unwrap();

                let row_len = doc.buffer().line_len(cursor_row as u32);

                if mode == Mode::Command {
                    print!(":{}", cmd.buffer().row_text(cmd.buffer().row_count() - 1));
                } else {
                    print!(
                        "{} {},{} rl:{} {}",
                        match mode {
                            Mode::Normal => "NORMAL",
                            Mode::Insert => "INSERT",
                            Mode::Visual => "VISUAL",
                            _ => "",
                        },
                        // scroll_x,
                        // scroll_y,
                        doc.selection().head().offset,
                        doc.selection().tail().offset,
                        // &doc.buffer().version().get(0), //
                        // &doc.buffer().replica_id(),
                        row_len,
                        pending_cmd
                    );
                }
            }

            if mode == Mode::Command {
                let cmd_text = cmd.buffer().row_text(cmd.buffer().row_count() - 1);
                let cmd_col = (cmd_text.chars().count() + 1) as u16;
                execute!(
                    stdout,
                    MoveTo(cmd_col, screen_rows as u16),
                    crossterm::cursor::SetCursorStyle::BlinkingBar,
                    crossterm::cursor::Show
                )
                .unwrap();
            } else {
                execute!(
                    stdout,
                    MoveTo(cursor_screen_col as u16, cursor_screen_row as u16),
                    match mode {
                        Mode::Insert => crossterm::cursor::SetCursorStyle::BlinkingBar,
                        _ => crossterm::cursor::SetCursorStyle::BlinkingBlock,
                    },
                    crossterm::cursor::Show
                )
                .unwrap();
            }

            stdout.flush().unwrap();
        }

        //------------------
        // input
        //------------------
        if event::poll(Duration::from_millis(50))? {
            let event = event::read()?;
            if let Event::Paste(content) = &event {
                if mode == Mode::Insert {
                    doc.apply_action(&Action::InsertText(content.clone()));
                    should_redraw = true;
                }
            }

            if let Event::Key(key_event) = event {
                should_redraw = false;

                let current_mode = mode.clone();

                // global actions
                match (key_event.code, key_event.modifiers) {
                    (KeyCode::Esc, _) => {
                        mode = Mode::Normal;
                        should_redraw = true;
                        if doc.has_selection() {
                            doc.apply_action(&Action::ClearCursors);
                        }
                    }
                    (KeyCode::Char('q'), KeyModifiers::CONTROL) => break,
                    _ => {}
                }

                let (count, _) = {
                    let mut count_str = String::new();
                    let mut parsing_count = true;
                    for ch in pending_cmd.chars() {
                        if parsing_count && ch.is_ascii_digit() {
                            count_str.push(ch);
                        } else {
                            parsing_count = false;
                        }
                    }
                    let count = count_str.parse::<u32>().unwrap_or(1);
                    (count, ())
                };

                let select = mode == Mode::Visual || mode == Mode::Visual_Line;
                let move_action = match (key_event.code, key_event.modifiers) {
                    (KeyCode::Left, _) => Action::MoveLeft { select, count },
                    (KeyCode::Right, _) => Action::MoveRight { select, count },
                    (KeyCode::Up, _) => Action::MoveUp { select, count },
                    (KeyCode::Down, _) => Action::MoveDown { select, count },
                    (KeyCode::PageUp, _) => Action::MoveUp {
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

                let normal_action = match (key_event.code, key_event.modifiers) {
                    (KeyCode::Esc, _) => {
                        pending_cmd.clear();
                        should_redraw = true;
                        Action::NoOp
                    }
                    (KeyCode::Char('i'), _) => {
                        if mode != Mode::Command {
                            mode = Mode::Insert;
                            should_redraw = true;
                        }
                        Action::NoOp
                    }
                    (KeyCode::Char('v'), _) => {
                        if mode != Mode::Command {
                            mode = Mode::Visual;
                            should_redraw = true;
                        }
                        Action::NoOp
                    }
                    (KeyCode::Char('V'), _) => {
                        if mode != Mode::Command {
                            mode = Mode::Visual_Line;
                            should_redraw = true;
                        }
                        Action::NoOp
                    }
                    (KeyCode::Char(':'), _) => {
                        if mode != Mode::Command {
                            mode = Mode::Command;
                            should_redraw = true;
                        }
                        Action::NoOp
                    }
                    (KeyCode::Char('r'), KeyModifiers::CONTROL) => Action::Redo { count },
                    (KeyCode::Char('u'), _) => Action::Undo { count },
                    (KeyCode::Char('h'), _) => Action::MoveLeft { select, count },
                    (KeyCode::Char('l'), _) => Action::MoveRight { select, count },
                    (KeyCode::Char('k'), _) => Action::MoveUp { select, count },
                    (KeyCode::Char('j'), _) => Action::MoveDown { select, count },
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
                    (KeyCode::Char(c), _) => {
                        pending_cmd.push(c);
                        let (count, cmd_without_count) = {
                            let mut count_str = String::new();
                            let mut cmd_str = String::new();
                            let mut parsing_count = true;
                            for ch in pending_cmd.chars() {
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
                                motion: Box::new(Action::MoveToStartOfLineNonSpace {
                                    select: true,
                                }),
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
                            "x" => Some(Action::Delete { count }),
                            "b" => Some(Action::MoveToPreviousWord { select, count }),
                            "w" => Some(Action::MoveToNextWord { select, count }),
                            "e" => Some(Action::MoveToNextWordEnd { select, count }),
                            "ge" => Some(Action::MoveToPreviousWordEnd { select, count }),
                            s if s.starts_with('f') && s.len() == 2 => {
                                let ch = s.chars().nth(1).unwrap();
                                Some(Action::FindCharacter {
                                    select,
                                    count,
                                    char: ch,
                                    forward: true,
                                })
                            }
                            s if s.starts_with('F') && s.len() == 2 => {
                                let ch = s.chars().nth(1).unwrap();
                                Some(Action::FindCharacter {
                                    select,
                                    count,
                                    char: ch,
                                    forward: false,
                                })
                            }
                            _ => None,
                        };

                        if let Some(a) = action {
                            pending_cmd.clear();
                            a
                        } else {
                            Action::NoOp
                        }
                    }
                    _ => Action::NoOp,
                };

                let _visual_action = Action::NoOp;

                // document actions
                let insert_action = match (key_event.code, key_event.modifiers) {
                    (KeyCode::Enter, _) if mode == Mode::Insert => Action::InsertNewLine,
                    (KeyCode::Tab, _) if mode == Mode::Insert || mode == Mode::Command => {
                        Action::InsertTab
                    }
                    (KeyCode::Delete, _) if mode == Mode::Insert || mode == Mode::Command => {
                        Action::Delete { count: 1 }
                    }
                    (KeyCode::Backspace, _) if mode == Mode::Insert || mode == Mode::Command => {
                        Action::Backspace
                    }
                    (KeyCode::Char(c), _) if mode == Mode::Insert || mode == Mode::Command => {
                        Action::InsertText(c.to_string())
                    }
                    _ => Action::NoOp,
                };

                let _command_action = if mode != Mode::Insert {
                    match (key_event.code, key_event.modifiers) {
                        _ => Action::NoOp,
                    }
                } else {
                    Action::NoOp
                };

                // loop!
                match current_mode {
                    Mode::Normal => {
                        if normal_action != Action::NoOp {
                            doc.apply_action(&normal_action);
                            pending_cmd.clear();
                        } else if move_action != Action::NoOp {
                            doc.apply_action(&move_action);
                            pending_cmd.clear();
                        }
                    }
                    Mode::Visual => {
                        if normal_action != Action::NoOp {
                            doc.apply_action(&normal_action);
                            pending_cmd.clear();
                        } else if move_action != Action::NoOp {
                            doc.apply_action(&move_action);
                            pending_cmd.clear();
                        }
                    }
                    Mode::Visual_Line => {
                        if normal_action != Action::NoOp {
                            doc.apply_action(&normal_action);
                            pending_cmd.clear();
                        } else if move_action != Action::NoOp {
                            doc.apply_action(&move_action);
                            pending_cmd.clear();
                        }
                    }
                    Mode::Insert => {
                        if insert_action != Action::NoOp {
                            doc.apply_action(&insert_action);
                        } else if move_action != Action::NoOp {
                            doc.apply_action(&move_action);
                            pending_cmd.clear();
                        }
                    }
                    Mode::Command => {
                        if let (KeyCode::Enter, _) = (key_event.code, key_event.modifiers) {
                            let command_text = cmd.buffer().row_text(0);
                            if let Ok(line_number) = command_text.trim().parse::<u32>() {
                                doc.apply_action(&Action::MoveToLine {
                                    select: false,
                                    line: line_number,
                                });
                            } else if command_text.trim() == "q" {
                                break;
                            }
                            // Clear command buffer and return to Normal mode
                            cmd = Document::new("").unwrap();
                            mode = Mode::Normal;
                            should_redraw = true;
                        } else if insert_action != Action::NoOp {
                            cmd.apply_action(&insert_action);
                        } else if let (KeyCode::Backspace, _) =
                            (key_event.code, key_event.modifiers)
                        {
                            cmd.apply_action(&Action::Backspace);
                        }
                    }
                }

                if !should_redraw {
                    if normal_action != Action::NoOp
                        || move_action != Action::NoOp
                        || insert_action != Action::NoOp
                    {
                        should_redraw = true;
                    }
                }
            } else {
                // do some background task here?
            }
        }
    }

    crossterm::terminal::disable_raw_mode().unwrap();
    execute!(
        stdout,
        crossterm::event::DisableBracketedPaste,
        Clear(ClearType::All),
        MoveTo(0, 0)
    )
    .unwrap();
    execute!(stdout, crossterm::cursor::Show).unwrap();

    Ok(())
}
