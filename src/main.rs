mod document;
mod highlight;

use document::Document;
use std::collections::HashMap;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Style, ThemeSet};
use syntect::parsing::SyntaxSet;

use highlight::Highlights;
use std::thread;
use std::time::Duration;

use crossterm::{
    cursor::MoveTo,
    event,
    event::{read, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{Clear, ClearType},
};
use std::io::{stdout, Write};

fn fill_to_eol(count: usize) {
    for _ in 0..count {
        print!(" ");
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let mut scroll_line: u32 = 0;

    let tab_size = 4;

    // Prepare default background pair
    let (fg, bg) = {
        let style = &hl.get_default_style();
        (
            crossterm::style::Color::Rgb {
                r: style.foreground.r,
                g: style.foreground.g,
                b: style.foreground.b,
            },
            crossterm::style::Color::Rgb {
                r: style.background.r,
                g: style.background.g,
                b: style.background.b,
            },
        )
    };

    crossterm::terminal::enable_raw_mode().unwrap();
    let mut stdout = stdout();

    loop {
        execute!(stdout, crossterm::cursor::Hide).unwrap();
        // execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap();
        // println!("Hello World!");

        let (screen_cols, screen_rows) = {
            let (cols, rows) = crossterm::terminal::size().unwrap();
            (cols as i32, rows as i32)
        };

        // refresh();
        // getmaxyx(stdscr(), &mut screen_rows, &mut screen_cols);

        let cursor = doc.cursor(0).unwrap().clone();
        let cursor_row = cursor.row as i32;
        let visible_rows = screen_rows - 1;

        let mut cursor_screen_row = cursor_row - scroll_line as i32;
        while cursor_screen_row >= visible_rows {
            scroll_line += 1;
            cursor_screen_row = cursor_row - scroll_line as i32;
        }
        while cursor_screen_row < 0 && scroll_line > 0 {
            scroll_line -= 1;
            cursor_screen_row = cursor_row - scroll_line as i32;
        }

        // render
        {
            execute!(stdout, MoveTo(0, 0)).unwrap();

            let buffer = doc.buffer();
            let total_rows = buffer.row_count();
            let end_line = (scroll_line + visible_rows as u32).min(total_rows);

            hl.highlight_lines(
                doc.buffer(),
                scroll_line as usize,
                (end_line - scroll_line) as usize,
            );

            let mut screen_row = 0;
            for row in scroll_line..end_line {
                execute!(stdout, MoveTo(0, screen_row)).unwrap();
                let text = doc.row_text(row) + " ";

                let mut ranges;
                if let Some(style_cache) = hl.render_line(row as usize) {
                    ranges = &style_cache.styles;
                    // use ranges safely here
                } else {
                    // handle missing case, maybe skip or default:
                    fill_to_eol(screen_cols as usize);
                    screen_row += 1;
                    continue;
                }

                let mut range_iter = ranges.iter();
                let mut current_range = range_iter.next();
                let mut range_remaining = current_range.map_or(0, |(_, s, e)| e - s);
                let mut current_style = current_range.map(|(style, _, _)| style);

                for (screen_col, ch) in text.chars().enumerate() {
                    if range_remaining == 0 {
                        current_range = range_iter.next();
                        range_remaining = current_range.map_or(0, |(_, s, e)| e - s);
                        current_style = current_range.map(|(style, _, _)| style);
                    }

                    if let Some(style) = current_style {
                        execute!(
                            stdout,
                            crossterm::style::SetForegroundColor(crossterm::style::Color::Rgb {
                                r: style.foreground.r,
                                g: style.foreground.g,
                                b: style.foreground.b
                            })
                        );

                        execute!(
                            stdout,
                            crossterm::style::SetBackgroundColor(crossterm::style::Color::Rgb {
                                r: style.background.r,
                                g: style.background.g,
                                b: style.background.b
                            })
                        );
                    }

                    if cursor.is_within(row, screen_col as u32) {
                        execute!(
                            stdout,
                            crossterm::style::SetBackgroundColor(crossterm::style::Color::Rgb {
                                r: 150,
                                g: 150,
                                b: 150
                            })
                        );
                    }

                    match ch {
                        '\t' => {
                            for _i in 0..tab_size {
                                print!(" ");
                            }
                        }
                        _ => {
                            print!("{}", ch);
                        }
                    }

                    range_remaining = range_remaining.saturating_sub(1);

                    if screen_col as i32 >= screen_cols {
                        break;
                    }
                }

                fill_to_eol((screen_cols - text.chars().count() as i32).max(0) as usize);

                screen_row += 1;
            }
        }

        // input
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key_event) = event::read()? {
                match (key_event.code, key_event.modifiers) {
                    (KeyCode::PageUp, _) => {
                        for _ in 0..screen_rows {
                            doc.move_up(false);
                        }
                    }
                    (KeyCode::PageDown, _) => {
                        for _ in 0..screen_rows {
                            doc.move_down(false);
                        }
                    }
                    (KeyCode::Left, KeyModifiers::CONTROL) => doc.move_to_previous_word(false),
                    (KeyCode::Right, KeyModifiers::CONTROL) => doc.move_to_next_word(false),
                    (KeyCode::Home, KeyModifiers::CONTROL) => doc.move_to_start_of_document(false),
                    (KeyCode::End, KeyModifiers::CONTROL) => doc.move_to_end_of_document(false),
                    (KeyCode::Home, KeyModifiers::NONE) => doc.move_to_start_of_line(false),
                    (KeyCode::End, KeyModifiers::NONE) => doc.move_to_end_of_line(false),
                    (KeyCode::Home, KeyModifiers::SHIFT) => doc.move_to_start_of_line(true),
                    (KeyCode::End, KeyModifiers::SHIFT) => doc.move_to_end_of_line(true),

                    (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                        if cursor.has_selection() {
                            let buffer = doc.buffer();
                            let sel = cursor.selection_text(buffer);
                            doc.select_next_same_word(&sel);
                        } else {
                            doc.select_current_word();
                        }
                    }

                    (KeyCode::Char('q'), KeyModifiers::CONTROL) => break, // CTRL+Q

                    (KeyCode::Esc, _) => doc.clear_cursors(),

                    (KeyCode::Up, KeyModifiers::NONE) => doc.move_up(false),
                    (KeyCode::Down, KeyModifiers::NONE) => doc.move_down(false),
                    (KeyCode::Left, KeyModifiers::NONE) => doc.move_left(false),
                    (KeyCode::Right, KeyModifiers::NONE) => doc.move_right(false),

                    (KeyCode::Up, KeyModifiers::SHIFT) => doc.move_up(true),
                    (KeyCode::Down, KeyModifiers::SHIFT) => doc.move_down(true),
                    (KeyCode::Left, KeyModifiers::SHIFT) => doc.move_left(true),
                    (KeyCode::Right, KeyModifiers::SHIFT) => doc.move_right(true),

                    // Special keys:
                    (KeyCode::Tab, _) => {
                        hl.update_from_line(doc.top_cursor_row() as usize);
                        for _ in 0..4 {
                            doc.insert_text(" ");
                            doc.move_right(false);
                        }
                    }

                    (KeyCode::Enter, _) => {
                        hl.update_from_line(doc.top_cursor_row() as usize);
                        doc.delete_text(0);
                        let newline = doc.new_line().to_string();
                        doc.insert_text(&newline);
                        doc.move_right(false);
                    }

                    (KeyCode::Backspace, _) => {
                        hl.update_from_line(doc.top_cursor_row() as usize);
                        if cursor.has_selection() {
                            doc.delete_text(0);
                        } else {
                            doc.move_left(false);
                            doc.delete_text(1);
                        }
                    }

                    (KeyCode::Delete, _) => {
                        hl.update_from_line(doc.top_cursor_row() as usize);
                        doc.delete_text(0);
                    }

                    (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                        hl.update_from_line(0);
                        doc.redo();
                    }

                    (KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                        hl.update_from_line(0);
                        doc.undo();
                    }

                    // Character input:
                    (KeyCode::Char(c), _) => {
                        hl.update_from_line(doc.top_cursor_row() as usize);
                        let has_selection = cursor.has_selection();
                        doc.delete_text(0);
                        doc.insert_text(&c.to_string());
                        doc.move_right(false);
                        if has_selection {
                            doc.move_left(false);
                        }
                    }

                    _ => {}
                }
            } else {
                // do some background here?
            }
        }
    }

    crossterm::terminal::disable_raw_mode().unwrap();
    // execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap();
    execute!(stdout, crossterm::cursor::Show).unwrap();

    Ok(())
}
