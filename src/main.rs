mod document;
use document::Document;
use ncurses::*;

use std::path::Path;
use syntect::dumps::{dump_to_file, from_dump_file};
use syntect::easy::{HighlightFile, HighlightLines};
use syntect::highlighting::{Style, Theme, ThemeSet, Color};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use std::io::BufRead;

use ncurses::*;
use std::collections::HashMap;

fn fill_to_eol(count: usize, pair_number: i16) {
    // Build the character with the desired color pair as background
    let space_ch = ' ' as u32 | COLOR_PAIR(pair_number) as u32;
    for _ in 0..count {
        addch(space_ch);
    }
}

fn rgb_to_ncurses_scale(value: u8) -> i16 {
    ((value as i32) * 1000 / 255) as i16
}

fn rgb_to_256color(r: u8, g: u8, b: u8) -> i16 {
    let r_index = (r as u16 * 5 / 255) as i16;
    let g_index = (g as u16 * 5 / 255) as i16;
    let b_index = (b as u16 * 5 / 255) as i16;
    16 + 36 * r_index + 6 * g_index + b_index
}

struct ColorPairManager {
    color_slots: HashMap<Color, i16>,
    next_color_slot: i16,
    pair_slots: HashMap<(Color, Color), i16>,
    next_pair_slot: i16,
}

impl ColorPairManager {
    fn new() -> Self {
        Self {
            color_slots: HashMap::new(),
            next_color_slot: 16,  // avoid overwriting default colors (0–7 or 0–15)
            pair_slots: HashMap::new(),
            next_pair_slot: 1,
        }
    }

    fn get_or_register_color(&mut self, color: Color) -> i16 {
        if let Some(&slot) = self.color_slots.get(&color) {
            return slot;
        }
        let slot = self.next_color_slot;
        if can_change_color() {
            init_color(
                slot,
                rgb_to_ncurses_scale(color.r),
                rgb_to_ncurses_scale(color.g),
                rgb_to_ncurses_scale(color.b),
            );
        }
        self.color_slots.insert(color, slot);
        self.next_color_slot += 1;
        slot
    }

    fn get_or_register_pair(&mut self, fg: Color, bg: Color) -> i16 {
        let key = (fg, bg);
        if let Some(&pair) = self.pair_slots.get(&key) {
            return pair;
        }
        let fg_slot = self.get_or_register_color(fg);
        let bg_slot = self.get_or_register_color(bg);
        // let fg_slot = rgb_to_256color(fg.r, fg.g, fg.b);
        // let bg_slot = rgb_to_256color(bg.r, bg.g, bg.b);
        let pair = self.next_pair_slot;
        init_pair(pair, fg_slot, bg_slot);
        self.pair_slots.insert(key, pair);
        self.next_pair_slot += 1;
        pair
    }
}


fn load_theme(tm_file: &str, enable_caching: bool) -> Theme {
    let tm_path = Path::new(tm_file);
    ThemeSet::get_theme(tm_path).unwrap()
}

fn main() {
    let ss = SyntaxSet::load_defaults_nonewlines();
    let ts = ThemeSet::load_defaults();
    // let theme = load_theme("/home/iceman/Developer/editors/tm-parser/test-cases/themes/Monokai.tmTheme", true);
    // let theme = load_theme("/home/iceman/Developer/editors/tm-parser/test-cases/themes/Abyss.tmTheme", true);
    let theme = &ts.themes["base16-ocean.dark"];

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

    // Get extension as lowercase string
    let extension = Path::new(&file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase()).unwrap();

    let syntax = ss.find_syntax_by_extension(&extension).unwrap();

    initscr();
    start_color();
    use_default_colors();

    raw();
    keypad(stdscr(), true);
    noecho();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    let mut manager = ColorPairManager::new();

    let mut last_ch = 0;
    let mut scroll_line: u32 = 0;

    
    loop {
        let mut h = HighlightLines::new(syntax, &theme);

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

        let mut last_pair_number = 0;

        // Render buffer
        {
            let buffer = doc.buffer();
            let total_rows = buffer.row_count();
            let end_line = (scroll_line + visible_rows as u32).min(total_rows);

            // highlight previous 10 lines
            let mut prior_lines = (scroll_line as i32) - 500;
            if prior_lines < 0 {
                prior_lines = 0
            }
            for row in prior_lines as u32..scroll_line {
                let text: String = doc.row_text(row) + " ";
                h.highlight_line(&text, &ss);
            }

            let mut screen_row = 0;
            for row in scroll_line..end_line {
                let text: String = doc.row_text(row) + " ";

                mv(screen_row, 0);

                let ranges: Vec<(Style, &str)> = h.highlight_line(&text, &ss).unwrap();

                // Create a flat iterator over ranges
                let mut range_iter = ranges.iter();
                let mut current_range = range_iter.next();
                let mut range_remaining = 0;
                let mut current_style = None;

                if let Some((style, substr)) = current_range {
                    range_remaining = substr.len();
                    current_style = Some(style);
                }

                // for (style, tt) in ranges {
                //     let pair_number = manager.get_or_register_pair(style.foreground, style.background);
                //     attron(COLOR_PAIR(pair_number));
                //     addstr(&text);
                //     attroff(COLOR_PAIR(pair_number));
                // }

                let mut screen_col = 0;

                // for ch in text.chars() {
                //     if cursor.is_within(row, screen_col as u32) {
                //         attron(A_REVERSE);
                //     }
                //     addch(ch as u32);
                //     attroff(A_REVERSE);
                //     screen_col += 1;
                //     if screen_col as i32 >= screen_cols {
                //         break;
                //     }
                // }

                for ch in text.chars() {
                    // If current range exhausted, move to next
                    if range_remaining == 0 {
                        current_range = range_iter.next();
                        if let Some((style, substr)) = current_range {
                            range_remaining = substr.len();
                            current_style = Some(style);
                        } else {
                            current_style = None;
                        }
                    }

                    // Apply style (if any)
                    if let Some(style) = current_style {
                        let pair_number = manager.get_or_register_pair(style.foreground, style.background);
                        last_pair_number = pair_number;
                        attron(COLOR_PAIR(pair_number));
                    }

                    // Draw cursor if needed
                    if cursor.is_within(row, screen_col as u32) {
                        attron(A_REVERSE);
                    }

                    addch(ch as u32);

                    if cursor.is_within(row, screen_col as u32) {
                        attroff(A_REVERSE);
                    }

                    if current_style.is_some() {
                        attroff(COLOR_PAIR(last_pair_number)); // Or use the same pair_number
                    }

                    screen_col += 1;
                    range_remaining = range_remaining.saturating_sub(1);

                    if screen_col as i32 >= screen_cols {
                        break;
                    }
                }

                fill_to_eol((screen_cols - screen_col) as usize, last_pair_number);
                // clrtoeol();

                screen_row += 1;
            }

            while screen_row < visible_rows {
                mv(screen_row, 0);
                fill_to_eol(screen_cols as usize, last_pair_number);
                // clrtoeol();
                screen_row += 1;
            }

            // Status bar
            let sel = doc.cursor(0).unwrap().selection_text(buffer);
            let _ = mvprintw(
                screen_rows - 1,
                0,
                &format!(
                    "Key: {}  Row:{} Col:{} [{}]",
                    last_ch, cursor.row, cursor.col, sel
                ),
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
            544 => doc.move_to_start_of_document(false), // CTRL+HOME
            539 => doc.move_to_end_of_document(false), // CTRL+END
            // 564 => , // CTRL+PPAGE
            // 559 => , // CTRL+NPAGE
            KEY_HOME => doc.move_to_start_of_line(false),
            KEY_END => doc.move_to_end_of_line(false),
            391 => doc.move_to_start_of_line(true), // SHIFT+HOME
            386 => doc.move_to_end_of_line(true),   // SHIFT+END
            4 => {
                // CTRL+D
                if cursor.has_selection() {
                    let buffer = doc.buffer();
                    let sel = doc.cursor(0).unwrap().selection_text(buffer);
                    doc.select_next_same_word(&sel)
                } else {
                    doc.select_current_word();
                }
            }
            17 => break, // CTRL+Q
            27 => {
                // ESCAPE
                doc.clear_cursors();
            }
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
                    9 => {
                        for _i in 0..4 {
                            doc.insert_text(&" ");
                            doc.move_right(false);
                        }
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
