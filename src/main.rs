mod document;

use ncurses::*;

use rope::Cursor;
use rope::Point;
use rope::Rope;
use text::Buffer;
use text::BufferId;
use text::ToOffset;

use std::ops::Range;

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use document::Document;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <file_path>", args[0]);
        return;
    }
    let file_path = &args[1];
    let mut file = match File::open(Path::new(file_path)) {
        Ok(file) => file,
        Err(err) => {
            println!("Error opening file {}: {}", file_path, err);
            return;
        }
    };

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
    start_color();

    // init_pair(1, COLOR_RED, COLOR_BLACK);
    curs_set(CURSOR_VISIBILITY::CURSOR_VISIBLE);

    let mut last_ch = 0;
    let mut line: u32 = 0;

    /* Wait for input. */
    loop {
        let mut screen_rows = 0;
        let mut screen_cols = 0;
        refresh();
        getmaxyx(stdscr(), &mut screen_rows, &mut screen_cols);

        // scroll
        let cursor = doc.cursor(0);
        let cursor_row = cursor.row as i32;
        let mut cursor_line: i32 = cursor_row - line as i32;
        let visible_rows = screen_rows - 1;
        while cursor_line >= visible_rows {
            line += 1;
            cursor_line = cursor_row - line as i32;
        }
        while cursor_line < 1 && line > 0 {
            line -= 1;
            cursor_line = cursor_row - line as i32;
        }
        if line < 1 {
            line = 0;
        }

        // render
        {
            let buffer = doc.buffer();
            let cur = cursor.normalized().sane(buffer);

            let rows = buffer.row_count();
            let mut last_line: u32 = line + visible_rows as u32;
            if last_line > rows {
                last_line = rows;
            }

            let mut screen_row = 0;
            for row in line .. last_line {
                let start = Point::new(row, 0).to_offset(&buffer);
                let end = Point::new(row, buffer.line_len(row)).to_offset(&buffer);
                let chunks = buffer.as_rope().chunks_in_range(start..end);
                let text = chunks.collect::<String>();
                
                // mvprintw(screen_row, 0, &text);
                
                mv(screen_row, 0);
                let mut screen_col = 0;
                for ch in text.chars() {
                    if cur.is_within(screen_row as u32, screen_col as u32) {
                        attron(A_REVERSE);
                    }
                    addch(ch as u32);
                    attroff(A_REVERSE);
                    screen_col += 1;
                }

                clrtoeol();
                screen_row += 1;
            }

            while screen_row < visible_rows {
                mv(screen_row, 0);
                clrtoeol();
                screen_row += 1;
            }

            // status bar
            {
                let cur = cursor.clone();
                mvprintw(screen_rows-1, 0, &format!("Key: {}", last_ch));
                // mvprintw(screen_rows-1, 0, &format!("a({} {})-({} {})", cur.anchor_row, cur.anchor_col, cur.row, cur.col));
                clrtoeol();
            }

            // cursor!
            mv((cur.row - line) as i32, cur.col as i32);
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
            564 => { // CTRL+KEY_PPAGE
                doc.move_to_start_of_document(false);
            }
            559 => { // CTRL+KEY_NPAGE
                doc.move_to_end_of_document(false);
            }
            KEY_HOME => {
                doc.move_to_start_of_line(false);
            }
            KEY_END => {
                doc.move_to_end_of_line(false);
            }
            391 => { // KEY_HOME+SHIFT
                doc.move_to_start_of_line(true);
            }
            386 => { // KEY_END+SHIFT
                doc.move_to_end_of_line(true);
            }
            17 => break, // CTRL+Q
            // 27 => break, // ESC key
            259 => {
                // UP
                doc.move_up(false);
            }
            258 => {
                // DOWN
                doc.move_down(false);
            }
            260 => {
                // LEFT
                doc.move_left(false);
            }
            261 => {
                // RIGHT
                doc.move_right(false);
            }
            337 => {
                // SHIFT+UP
                doc.move_up(true);
            }
            336 => {
                // SHIFT+DOWN
                doc.move_down(true);
            }
            393 => {
                // SHIFT+LEFT
                doc.move_left(true);
            }
            402 => {
                // SHIFT+RIGHT
                doc.move_right(true);
            }
            // You can handle other keys here
            ch => {
                last_ch = ch;

                match ch {
                    ch if ch >= 32 && ch < 127 => {
                        let s = (ch as u8 as char).to_string();
                        doc.delete_text(0); // deletes selected text
                        doc.insert_text(&s);
                        doc.move_right(false);
                    }
                    10 => {
                        // new line
                        doc.delete_text(0); // deletes selected text
                        doc.insert_text(&"\n");
                        doc.move_right(false);
                    }
                    263 => {
                        // backspace
                        if (cursor.has_selection()) {
                            doc.delete_text(0); // deletes selected text
                        } else {
                            doc.move_left(false);
                            doc.delete_text(1);
                        }
                    }
                    330 => {
                        // delete
                        doc.delete_text(0);
                    }
                    18 => {
                        doc.redo();
                    }
                    26 => {
                        doc.undo();
                    }
                    _ => {}
                }
            }
        }
    }

    endwin();
}
