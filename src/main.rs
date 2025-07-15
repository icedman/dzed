mod document;

use ncurses::*;

use document::Document;
use rope::Point;
use text::{Buffer, ToOffset};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <file_path>", args[0]);
        return;
    }

    let file_path = &args[1];
    let mut doc = match Document::new(file_path) {
        Ok(doc) => doc,
        Err(err) => {
            println!("Failed to open document: {}", err);
            return;
        }
    };

    initscr();
    raw();
    keypad(stdscr(), true);
    noecho();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    let mut last_ch = 0;
    let mut scroll_line: u32 = 0;

    loop {
        let (mut screen_rows, mut screen_cols) = (0, 0);
        refresh();
        getmaxyx(stdscr(), &mut screen_rows, &mut screen_cols);

        let cursor = doc.cursor(0).unwrap().clone();
        let cursor_row = cursor.row as i32;
        let mut cursor_screen_row = cursor_row - scroll_line as i32;

        let visible_rows = screen_rows - 1;

        while cursor_screen_row >= visible_rows {
            scroll_line += 1;
            cursor_screen_row = cursor_row - scroll_line as i32;
        }
        while cursor_screen_row < 0 && scroll_line > 0 {
            scroll_line -= 1;
            cursor_screen_row = cursor_row - scroll_line as i32;
        }

        // Render buffer
        {
            let buffer = doc.buffer();
            let total_rows = buffer.row_count();
            let end_line = (scroll_line + visible_rows as u32).min(total_rows);

            let mut screen_row = 0;
            for row in scroll_line..end_line {
                let text: String = doc.row_text(row);

                mv(screen_row, 0);
                let mut screen_col = 0;

                for ch in text.chars() {
                    if cursor.is_within(row, screen_col as u32) {
                        attron(A_REVERSE);
                    }
                    addch(ch as u32);
                    attroff(A_REVERSE);
                    screen_col += 1;
                    if screen_col as i32 >= screen_cols {
                        break;
                    }
                }

                clrtoeol();
                screen_row += 1;
            }

            while screen_row < visible_rows {
                mv(screen_row, 0);
                clrtoeol();
                screen_row += 1;
            }

            // Status bar
            mvprintw(
                screen_rows - 1,
                0,
                &format!("Key: {}  Row:{} Col:{}", last_ch, cursor.row, cursor.col),
            );
            clrtoeol();
        }

        match getch() {
            KEY_PPAGE => {
                for _ in 0..screen_rows {
                    doc.move_up(false);
                }
            }
            KEY_NPAGE => {
                for _ in 0..screen_rows {
                    doc.move_down(false);
                }
            }
            554 => doc.move_to_previous_word(false), // CTRL+LEFT
            569 => doc.move_to_next_word(false),     // CTRL+RIGHT
            564 => doc.move_to_start_of_document(false), // CTRL+PPAGE
            559 => doc.move_to_end_of_document(false), // CTRL+NPAGE
            KEY_HOME => doc.move_to_start_of_line(false),
            KEY_END => doc.move_to_end_of_line(false),
            391 => doc.move_to_start_of_line(true), // SHIFT+HOME
            386 => doc.move_to_end_of_line(true),   // SHIFT+END
            4 => doc.select_current_word(),         // CTRL+D
            17 => break,                            // CTRL+Q

            259 => doc.move_up(false),    // UP
            258 => doc.move_down(false),  // DOWN
            260 => doc.move_left(false),  // LEFT
            261 => doc.move_right(false), // RIGHT

            337 => doc.move_up(true),    // SHIFT+UP
            336 => doc.move_down(true),  // SHIFT+DOWN
            393 => doc.move_left(true),  // SHIFT+LEFT
            402 => doc.move_right(true), // SHIFT+RIGHT

            ch => {
                last_ch = ch;
                match ch {
                    ch if ch >= 32 && ch < 127 => {
                        let s = (ch as u8 as char).to_string();
                        doc.delete_text(0); // delete selection if any
                        doc.insert_text(&s);
                        doc.move_right(false);
                    }
                    10 => {
                        doc.delete_text(0);
                        let new_line = doc.new_line().to_string();
                        doc.insert_text(&new_line);
                        doc.move_right(false);
                    }
                    263 => {
                        // Backspace
                        if cursor.has_selection() {
                            doc.delete_text(0);
                        } else {
                            doc.move_left(false);
                            doc.delete_text(1);
                        }
                    }
                    330 => doc.delete_text(0), // Delete key
                    18 => doc.redo(),          // CTRL+R
                    26 => doc.undo(),          // CTRL+Z
                    _ => {}
                }
            }
        }
    }

    endwin();
}
