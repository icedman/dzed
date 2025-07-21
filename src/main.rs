mod actions;
mod document;
mod highlight;

use actions::Action;
use document::Document;
use std::collections::HashMap;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use text::{Buffer, BufferId};

use highlight::Highlights;
use std::thread;
use std::time::Duration;

use crossterm::{
    cursor::MoveTo,
    event,
    event::{read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{Clear, ClearType},
};
use std::io::{stdout, Write};

pub trait ToCrossTerm {
    fn rgb(&self) -> crossterm::style::Color;
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
            a: self.a
        }
    }
}

fn fill_to_eol(count: usize) {
    for _ in 0..count {
        print!(" ");
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = stdout();
    crossterm::terminal::enable_raw_mode().unwrap();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap();

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
            bg.rgb(),
            settings.caret.unwrap_or(fg).rgb(),
            settings.line_highlight.unwrap_or(bg.lighten(10)).rgb(),
            settings.selection.unwrap_or(bg.lighten(10)).rgb(),
            settings.gutter.unwrap_or(bg.darken(10)).rgb(),
        )
    };

    let mut dirty_hl = true;

    loop {
        execute!(stdout, crossterm::cursor::Hide).unwrap();

        let (screen_cols, screen_rows) = {
            let (cols, rows) = crossterm::terminal::size().unwrap();
            (cols as i32, rows as i32)
        };

        let cursor = doc.cursor(0).unwrap().clone();
        let cursor_row = cursor.row as i32;
        let visible_rows = screen_rows - 1;

        // scroll
        let mut cursor_screen_row = cursor_row - scroll_y as i32;
        while cursor_screen_row >= visible_rows {
            scroll_y += 1;
            cursor_screen_row = cursor_row - scroll_y as i32;
        }
        while cursor_screen_row < 0 && scroll_y > 0 {
            scroll_y -= 1;
            cursor_screen_row = cursor_row - scroll_y as i32;
        }

        // bar
        // print!("\x1b[5 q");

        // render
        {
            execute!(stdout, MoveTo(0, 0)).unwrap();
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
                let text = doc.row_text(row) + " ";

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

                let mut range_iter = ranges.iter();
                let mut current_range = range_iter.next();
                let mut range_remaining = current_range.map_or(0, |(_, s, e)| e - s);
                let mut current_style = current_range.map(|(style, _, _)| style);

                let mut screen_col = 0;
                for (_, ch) in text.chars().enumerate() {
                    if range_remaining == 0 {
                        current_range = range_iter.next();
                        range_remaining = current_range.map_or(0, |(_, s, e)| e - s);
                        current_style = current_range.map(|(style, _, _)| style);
                    }

                    if let Some(style) = current_style {
                        execute!(
                            stdout,
                            crossterm::style::SetForegroundColor(style.foreground.rgb())
                        )
                        .unwrap();

                        execute!(
                            stdout,
                            crossterm::style::SetBackgroundColor(style.background.rgb())
                        )
                        .unwrap();
                    }

                    // within selection
                    if cursor.is_within(row, screen_col as u32) {
                        execute!(stdout, crossterm::style::SetBackgroundColor(clr_select)).unwrap();
                    }

                    match ch {
                        '\t' => {
                            for _i in 0..tab_size {
                                print!(" ");
                                if !cursor.is_within(row, (screen_col + 1) as u32) {
                                    execute!(stdout, crossterm::style::SetBackgroundColor(clr_bg)).unwrap();
                                }
                            }
                        }
                        _ => {
                            print!("{}", ch);
                        }
                    }

                    range_remaining = range_remaining.saturating_sub(1);

                    screen_col += 1;
                    if screen_col as i32 >= screen_cols {
                        break;
                    }
                }

                execute!(stdout, crossterm::style::SetBackgroundColor(clr_bg));
                fill_to_eol((screen_cols - text.chars().count() as i32).max(0) as usize);

                screen_row += 1;
                if screen_row + 1 > screen_rows as u16 {
                    break;
                }
            }

            // statusbar
            {
                execute!(stdout, MoveTo(0, screen_rows as u16)).unwrap();
                execute!(stdout, crossterm::style::SetBackgroundColor(clr_gutter));
                fill_to_eol(screen_cols as usize);
            }

            std::io::stdout().flush().unwrap();
        }

        // input
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key_event) = event::read()? {
                // global actions
                match (key_event.code, key_event.modifiers) {
                    (KeyCode::Char('q'), KeyModifiers::CONTROL) => break,
                    _ => {}
                }

                // document actions
                let action = match (key_event.code, key_event.modifiers) {
                    (KeyCode::PageUp, _) => Action::MoveUp {
                        select: false,
                        count: screen_rows as usize,
                    },
                    (KeyCode::PageDown, _) => Action::MoveDown {
                        select: false,
                        count: screen_rows as usize,
                    },

                    (KeyCode::Left, KeyModifiers::CONTROL) => {
                        Action::MoveToPreviousWord { select: false }
                    }
                    (KeyCode::Right, KeyModifiers::CONTROL) => {
                        Action::MoveToNextWord { select: false }
                    }

                    (KeyCode::Home, KeyModifiers::CONTROL) => {
                        Action::MoveToStartOfDocument { select: false }
                    }
                    (KeyCode::End, KeyModifiers::CONTROL) => {
                        Action::MoveToEndOfDocument { select: false }
                    }

                    (KeyCode::Home, KeyModifiers::NONE) => {
                        Action::MoveToStartOfLine { select: false }
                    }
                    (KeyCode::End, KeyModifiers::NONE) => Action::MoveToEndOfLine { select: false },

                    (KeyCode::Home, KeyModifiers::SHIFT) => {
                        Action::MoveToStartOfLine { select: true }
                    }
                    (KeyCode::End, KeyModifiers::SHIFT) => Action::MoveToEndOfLine { select: true },

                    (KeyCode::Left, KeyModifiers::NONE) => Action::MoveLeft { select: false },
                    (KeyCode::Right, KeyModifiers::NONE) => Action::MoveRight { select: false },
                    (KeyCode::Left, KeyModifiers::SHIFT) => Action::MoveLeft { select: true },
                    (KeyCode::Right, KeyModifiers::SHIFT) => Action::MoveRight { select: true },

                    (KeyCode::Up, KeyModifiers::NONE) => Action::MoveUp {
                        select: false,
                        count: 1,
                    },
                    (KeyCode::Down, KeyModifiers::NONE) => Action::MoveDown {
                        select: false,
                        count: 1,
                    },
                    (KeyCode::Up, KeyModifiers::SHIFT) => Action::MoveUp {
                        select: true,
                        count: 1,
                    },
                    (KeyCode::Down, KeyModifiers::SHIFT) => Action::MoveDown {
                        select: true,
                        count: 1,
                    },
                    (KeyCode::Char('d'), KeyModifiers::CONTROL) => Action::SelectCurrentWord,
                    (KeyCode::Esc, _) => Action::ClearCursors,

                    (KeyCode::Tab, _) => Action::InsertTab,
                    (KeyCode::Enter, _) => Action::InsertNewLine,

                    (KeyCode::Backspace, _) => Action::Backspace,
                    (KeyCode::Delete, _) => Action::Delete,

                    (KeyCode::Char('r'), KeyModifiers::CONTROL) => Action::Redo,
                    (KeyCode::Char('z'), KeyModifiers::CONTROL) => Action::Undo,

                    (KeyCode::Char(c), _) => Action::InsertText(c.to_string()),

                    _ => {
                        dirty_hl = false;
                        Action::NoOp
                    }
                };

                doc.apply_action(&action);
            } else {
                // do some background here?
            }
        }
    }

    crossterm::terminal::disable_raw_mode().unwrap();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap();
    execute!(stdout, crossterm::cursor::Show).unwrap();

    Ok(())
}

fn _main() {
    let s = "* 🍐✅ *";
    let mut v = Vec::new();
    let mut idx = 0;
    for c in s.chars() {
        v.push(idx);
        idx += c.len_utf8();
    }

    let mut buffer = Buffer::new(0, BufferId::new(1).unwrap(), s);
    let s = 3;
    let e = s + 0;
    buffer.edit([(v[s]..v[e], "abcd")]);
    let chars = buffer.chars_at(rope::Point::new(0, 0));
    println!("{}", chars.collect::<String>());
}