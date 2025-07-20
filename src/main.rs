mod document;
mod highlight;

use document::Document;
use ncurses::*;
use std::collections::HashMap;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Style, ThemeSet};
use syntect::parsing::SyntaxSet;

use highlight::Highlights;
use std::thread;
use std::time::Duration;

fn fill_to_eol(count: usize, pair_number: i16) {
    let space_ch = ' ' as u32 | COLOR_PAIR(pair_number) as u32;
    for _ in 0..count {
        addch(space_ch);
    }
}

fn rgb_to_ncurses_scale(value: u8) -> i16 {
    ((value as i32) * 1000 / 255) as i16
}

struct ColorPairManager {
    color_slots: HashMap<Color, i16>,
    pair_slots: HashMap<(Color, Color), i16>,
    next_color_slot: i16,
    next_pair_slot: i16,
}

impl ColorPairManager {
    fn new() -> Self {
        Self {
            color_slots: HashMap::new(),
            pair_slots: HashMap::new(),
            next_color_slot: 16,
            next_pair_slot: 1,
        }
    }

    fn get_or_register_color(&mut self, color: Color) -> i16 {
        *self.color_slots.entry(color).or_insert_with(|| {
            let slot = self.next_color_slot;
            if can_change_color() {
                init_color(
                    slot,
                    rgb_to_ncurses_scale(color.r),
                    rgb_to_ncurses_scale(color.g),
                    rgb_to_ncurses_scale(color.b),
                );
            }
            self.next_color_slot += 1;
            slot
        })
    }

    fn get_or_register_pair(&mut self, fg: Color, bg: Color) -> i16 {
        let key = (fg, bg);

        if let Some(&pair) = self.pair_slots.get(&key) {
            return pair;
        }

        let fg_slot = self.get_or_register_color(fg);
        let bg_slot = self.get_or_register_color(bg);

        let pair = self.next_pair_slot;
        init_pair(pair, fg_slot, bg_slot);

        self.pair_slots.insert(key, pair);
        self.next_pair_slot += 1;

        pair
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        return;
    }

    let file_path = &args[1];
    let mut doc = match Document::new(file_path) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("Failed to open document: {}", err);
            return;
        }
    };

    let mut hl = Highlights::new(file_path);

    // Setup ncurses
    initscr();
    start_color();
    use_default_colors();
    raw();
    keypad(stdscr(), true);
    noecho();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    nodelay(stdscr(), true);

    let mut manager = ColorPairManager::new();
    let mut last_ch = 0;
    let mut scroll_line: u32 = 0;

    let tab_size = 4;

    // Prepare default background pair
    let default_pair_number = {
        let style = &hl.get_default_style();
        manager.get_or_register_pair(style.foreground, style.background)
    };

    loop {
        let (mut screen_rows, mut screen_cols) = (0, 0);
        refresh();
        getmaxyx(stdscr(), &mut screen_rows, &mut screen_cols);

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

        // Render buffer
        {
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
                mv(screen_row, 0);
                let text = doc.row_text(row) + " ";

                let mut ranges;
                if let Some(style_cache) = hl.render_line(row as usize) {
                    ranges = &style_cache.styles;
                    // use ranges safely here
                } else {
                    // handle missing case, maybe skip or default:
                    fill_to_eol(screen_cols as usize, default_pair_number);
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
                        let pair_number =
                            manager.get_or_register_pair(style.foreground, style.background);
                        attron(COLOR_PAIR(pair_number));
                    }

                    if cursor.is_within(row, screen_col as u32) {
                        attron(A_REVERSE);
                    }

                    match ch {
                        '\t' => {
                            for _i in 0..tab_size {
                                addch(' ' as u32);
                                attroff(A_REVERSE);
                            }
                        }
                        _ => {
                            addch(ch as u32);
                        }
                    }

                    if cursor.is_within(row, screen_col as u32) {
                        attroff(A_REVERSE);
                    }

                    range_remaining = range_remaining.saturating_sub(1);

                    if screen_col as i32 >= screen_cols {
                        break;
                    }
                }

                fill_to_eol(
                    (screen_cols - text.chars().count() as i32).max(0) as usize,
                    default_pair_number,
                );

                screen_row += 1;
            }

            // Clear remaining lines
            while screen_row < visible_rows + 1 {
                mv(screen_row, 0);
                fill_to_eol(screen_cols as usize, default_pair_number);
                screen_row += 1;
            }

            let (hl_cache_size, hl_start) = hl.stats();

            // Status bar
            attron(COLOR_PAIR(default_pair_number));
            mvprintw(
                screen_rows - 1,
                0,
                &format!(
                    "[{}]  {},{} hl: {} {}",
                    last_ch,
                    cursor.row,
                    cursor.col,
                    hl_cache_size,
                    hl_start,
                    // doc.cursor(0).unwrap().selection_text(buffer)
                ),
            );
            attroff(COLOR_PAIR(default_pair_number));
        }

        let mut ch = 0;
        loop {
            ch = getch();
            if ch != ERR {
                break;
            }
            // background
            thread::sleep(Duration::from_millis(10)); // 10ms pause (~100 FPS)
        }

        match ch {
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
            544 => doc.move_to_start_of_document(false),
            539 => doc.move_to_end_of_document(false),
            KEY_HOME => doc.move_to_start_of_line(false),
            KEY_END => doc.move_to_end_of_line(false),
            391 => doc.move_to_start_of_line(true), // SHIFT+HOME
            386 => doc.move_to_end_of_line(true),   // SHIFT+END
            4 => {
                // CTRL+D
                if cursor.has_selection() {
                    let buffer = doc.buffer(); // Fetch buffer here
                    let sel = cursor.selection_text(buffer);
                    doc.select_next_same_word(&sel)
                } else {
                    doc.select_current_word();
                }
            }
            17 => break,               // CTRL+Q
            27 => doc.clear_cursors(), // ESCAPE
            259 => doc.move_up(false),
            258 => doc.move_down(false),
            260 => doc.move_left(false),
            261 => doc.move_right(false),
            337 => doc.move_up(true),    // SHIFT+UP
            336 => doc.move_down(true),  // SHIFT+DOWN
            393 => doc.move_left(true),  // SHIFT+LEFT
            402 => doc.move_right(true), // SHIFT+RIGHT
            ch => {
                last_ch = ch;
                match ch {
                    32..=126 => {
                        // insert
                        hl.update_from_line(doc.top_cursor_row() as usize);
                        let s = (ch as u8 as char).to_string();
                        let has_selection = cursor.has_selection();
                        doc.delete_text(0);
                        doc.insert_text(&s);
                        doc.move_right(false);
                        if has_selection {
                            doc.move_left(false);
                        }
                    }
                    9 => {
                        // tab
                        hl.update_from_line(doc.top_cursor_row() as usize);
                        for _ in 0..4 {
                            doc.insert_text(" ");
                            doc.move_right(false);
                        }
                    }
                    10 => {
                        // newline
                        hl.update_from_line(doc.top_cursor_row() as usize);
                        doc.delete_text(0);
                        let newline = doc.new_line().to_string();
                        doc.insert_text(&newline);
                        doc.move_right(false);
                    }
                    263 => {
                        // Backspace
                        hl.update_from_line(doc.top_cursor_row() as usize);
                        if cursor.has_selection() {
                            doc.delete_text(0);
                        } else {
                            doc.move_left(false);
                            doc.delete_text(1);
                        }
                    }
                    330 => {
                        // Delete key
                        hl.update_from_line(doc.top_cursor_row() as usize);
                        doc.delete_text(0);
                    }
                    18 => {
                        // CTRL+R
                        hl.update_from_line(0);
                        doc.redo()
                    }
                    26 => {
                        // CTRL+Z
                        hl.update_from_line(0);
                        doc.undo()
                    }
                    _ => {}
                }
            }
        }
    }

    endwin();
}
