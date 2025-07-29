mod actions;
mod document;
mod highlight;
mod selections;

use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::Debug,
    io::{stdout, Result, Write},
    ops::Range,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use crossterm::{
    cursor::MoveTo,
    event::{self, read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{Clear, ClearType},
};

use rope::Point;
use sum_tree::Bias;
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, Style, ThemeSet},
    parsing::SyntaxSet,
};
use text::{
    Anchor, AnchorRangeExt, Buffer, BufferId, BufferSnapshot, Selection, SelectionGoal, ToOffset,
    ToPoint,
};

use actions::{Action, Mode};
use document::{BufferText, Document};
use highlight::Highlights;
use selections::SelectionCollection;

use clock;

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
        Err(err) => {
            return Ok(());
        }
    };
    let mut pending_cmd = "".to_string();
    let mut mode = Mode::Normal;

    let mut stdout = stdout();
    crossterm::terminal::enable_raw_mode().unwrap();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap();

    let mut hl = Highlights::new(file_path);
    let mut scroll_x: u32 = 0;
    let mut scroll_y: u32 = 0;

    let tab_size = 4;

    // Prepare default background pair
    let (clr_fg, clr_bg, clr_caret, clr_current_line, clr_select, clr_gutter) = {
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

    let mut paste_buffer = String::new();
    let mut last_char_time = Instant::now();
    let paste_timeout = Duration::from_millis(5); // threshold to separate pastes from normal typing

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

        // get cursor information
        let cursor = doc.selection();
        let cursor_head = cursor.head();
        let cursor_tail = cursor.tail();
        let mut cursor_range = if cursor_head.cmp(&cursor_tail, &doc.buffer()) == Ordering::Less {
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
        let cursor_row = cursor_point.row as i32;
        let cursor_col = cursor_point.column as i32;
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

            // execute!(stdout, MoveTo(0, 0)).unwrap();
            // execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap();

            let buffer = doc.buffer();
            let total_rows = buffer.row_count();
            let end_line = (scroll_y + visible_rows as u32).min(total_rows);

            if dirty_hl {
                hl.highlight_lines(
                    doc.buffer(),
                    scroll_y as usize,
                    (end_line - scroll_y) as usize,
                );
            }
            dirty_hl = true;

            let mut screen_row = 0;
            for row in scroll_y..end_line {
                execute!(stdout, MoveTo(0, screen_row)).unwrap();
                let text = doc.buffer().row_text(row) + " ";

                let mut ranges;
                if let Some(style_cache) = hl.render_line(row as usize) {
                    ranges = &style_cache.styles;
                    // use ranges safely here
                } else {
                    // handle missing case, maybe skip or default:
                    execute!(stdout, crossterm::style::SetBackgroundColor(clr_bg)).unwrap();
                    fill_to_eol(screen_cols as usize);
                    screen_row += 1;
                    continue;
                }

                // style range
                let mut range_iter = ranges.iter();
                let mut current_range = range_iter.next();
                let mut range_remaining = current_range.map_or(0, |(_, s, e)| e - s);
                let mut current_style = current_range.map(|(style, _, _)| style);

                let mut x_scroll = scroll_x;
                let mut cols_remaining = screen_cols;
                let at_cursor_row = cursor_row == row as i32;

                let mut rc = 0;
                for (column, ch) in text.chars().enumerate() {
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

                    if at_cursor_row {
                        // bg = clr_current_line.clone();
                    }

                    let (selected, mut selected_line, at_cursor) =
                        doc.selections().is_selected(row, rc, &buffer);
                    if selected {
                        bg = clr_select;
                    }
                    selected_line = selected_line && mode == Mode::Visual_Line;
                    if selected_line {
                        bg = clr_select;
                    }
                    if at_cursor {
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
                                    if at_cursor {
                                        execute!(
                                            stdout,
                                            crossterm::style::SetBackgroundColor(clr_bg)
                                        )
                                        .unwrap();
                                    }
                                    cols_remaining = cols_remaining.saturating_sub(1);
                                }
                                rc += ch.len_utf8() as u32;
                            }
                            _ => {
                                // let fc = if ch == ' ' && at_cursor { '_' } else { ch };
                                print!("{}", ch);
                                cols_remaining = cols_remaining.saturating_sub(1);
                                rc += ch.len_utf8() as u32;
                            }
                        }
                    }

                    range_remaining = range_remaining.saturating_sub(1);

                    if cols_remaining <= 0 {
                        break;
                    }
                }

                execute!(
                    stdout,
                    crossterm::style::SetBackgroundColor({
                        // if at_cursor_row {
                        //     clr_current_line
                        // } else {
                        clr_bg
                        // }
                    })
                );

                // fill_to_eol((screen_cols - text.chars().count() as i32).max(0) as usize);
                fill_to_eol(cols_remaining.max(0) as usize);

                screen_row += 1;
                if screen_row + 1 > screen_rows as u16 {
                    break;
                }
            }

            // statusbar
            {
                execute!(stdout, crossterm::style::SetForegroundColor(clr_fg));
                execute!(stdout, crossterm::style::SetBackgroundColor(clr_gutter));
                execute!(stdout, MoveTo(0, screen_rows as u16)).unwrap();
                fill_to_eol(screen_cols as usize);
                execute!(stdout, MoveTo(0, screen_rows as u16)).unwrap();

                let row_len = doc.buffer().line_len(cursor_row as u32);

                if mode == Mode::Command {
                    print!(":{}", cmd.buffer().row_text(cmd.buffer().row_count() - 1));
                } else {
                    print!(
                        "{} {},{} v:{} rl:{} {}",
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
                        &doc.buffer().version().get(0), // &doc.buffer().replica_id()
                        row_len,
                        pending_cmd
                    );
                }
            }

            stdout.flush().unwrap();
        }

        //------------------
        // input
        //------------------
        if event::poll(Duration::from_millis(50))? {
            // match event {
            if let Event::Key(key_event) = event::read()? {
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

                let mut action = Action::NoOp;
                let mut select = mode == Mode::Visual || mode == Mode::Visual_Line;
                let move_action = match (key_event.code, key_event.modifiers) {
                    (KeyCode::Left, _) => Action::MoveLeft { select },
                    (KeyCode::Right, _) => Action::MoveRight { select },
                    (KeyCode::Up, _) => Action::MoveUp { select },
                    (KeyCode::Down, _) => Action::MoveDown { select },
                    (KeyCode::PageUp, _) => Action::MoveUpCount {
                        select,
                        count: (visible_rows >> 1) as u32,
                    },
                    (KeyCode::PageDown, _) => Action::MoveDownCount {
                        select,
                        count: (visible_rows >> 1) as u32,
                    },
                    (KeyCode::Home, _) => Action::MoveToStartOfLine { select },
                    (KeyCode::End, _) => Action::MoveToEndOfLine { select },
                    (KeyCode::Char('{'), _) => Action::MoveToPreviousParagraph { select },
                    (KeyCode::Char('}'), _) => Action::MoveToNextParagraph { select },
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
                    (KeyCode::Char('r'), KeyModifiers::CONTROL) => Action::Redo,
                    (KeyCode::Char('u'), _) => Action::Undo,
                    (KeyCode::Char('h'), _) => Action::MoveLeft { select },
                    (KeyCode::Char('l'), _) => Action::MoveRight { select },
                    (KeyCode::Char('k'), _) => Action::MoveUp { select },
                    (KeyCode::Char('j'), _) => Action::MoveDown { select },
                    (KeyCode::Delete, _) => Action::DeleteText { count: 1 },
                    (KeyCode::Backspace, _) => Action::MoveLeft { select },
                    (KeyCode::Left, KeyModifiers::SHIFT) => {
                        Action::MoveToPreviousWord { select: false }
                    }
                    (KeyCode::Right, KeyModifiers::SHIFT) => {
                        Action::MoveToNextWord { select: false }
                    }
                    (KeyCode::Char(c), _) => {
                        pending_cmd.push(c);
                        match pending_cmd {
                            ref s if s.ends_with("gg") => {
                                pending_cmd.clear();
                                Action::MoveToStartOfDocument { select }
                            }
                            ref s if s.ends_with("G") => {
                                pending_cmd.clear();
                                Action::MoveToEndOfDocument { select }
                            }
                            ref s if s.ends_with("dd") => {
                                pending_cmd.clear();
                                Action::DeleteCurrentLine
                            }
                            ref s if s.ends_with("dw") => {
                                pending_cmd.clear();
                                Action::DeleteCurrentWord
                            }
                            ref s if s.ends_with("x") => {
                                pending_cmd.clear();
                                Action::Delete
                            }
                            ref s if s.ends_with("b") => {
                                pending_cmd.clear();
                                Action::MoveToPreviousWord { select }
                            }
                            ref s if s.ends_with("w") => {
                                pending_cmd.clear();
                                Action::MoveToNextWord { select }
                            }
                            _ => Action::NoOp,
                        }
                    }
                    _ => Action::NoOp,
                };

                let visual_action = Action::NoOp;

                // document actions
                let insert_action = if mode == Mode::Insert || mode == Mode::Command {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Enter, _) => Action::InsertNewLine,
                        (KeyCode::Tab, _) => Action::InsertTab,
                        (KeyCode::Delete, _) => Action::Delete,
                        (KeyCode::Backspace, _) => Action::Backspace,
                        (KeyCode::Char(c), _) => Action::InsertText(c.to_string()),
                        _ => Action::NoOp,
                    }
                } else {
                    Action::NoOp
                };

                let command_action = if mode != Mode::Insert {
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
                        } else if move_action != Action::NoOp {
                            doc.apply_action(&move_action);
                            pending_cmd.clear();
                        }
                    }
                    Mode::Visual => {
                        if normal_action != Action::NoOp {
                            doc.apply_action(&normal_action);
                        } else if move_action != Action::NoOp {
                            doc.apply_action(&move_action);
                            pending_cmd.clear();
                        }
                    }
                    Mode::Visual_Line => {
                        if normal_action != Action::NoOp {
                            doc.apply_action(&normal_action);
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
                        if insert_action != Action::NoOp {
                            cmd.apply_action(&insert_action);
                        } else if move_action != Action::NoOp {
                            //cmd.apply_action(&move_action);
                        }
                    }
                    _ => {}
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
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap();
    execute!(stdout, crossterm::cursor::Show).unwrap();

    Ok(())
}
