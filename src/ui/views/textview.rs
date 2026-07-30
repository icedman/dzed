use crate::controller::actions::Mode;
use crate::editor::display::display_map::DisplayPoint;
use crate::editor::{Editor, document::BufferText};
use crate::services::search::TextSearch;
use crate::ui::layout::Rect;
use crate::ui::theme::ToCrossTerm;
use crate::ui::views::View;

use std::io::Write;

use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};

pub struct TextView {}

impl TextView {
    pub fn new() -> Self {
        TextView {}
    }
}

impl TextView {
    fn draw_textview<W: Write>(
        &self,
        w: &mut W,
        inner_rect: Rect,
        editor: &Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (screen_cols, _) = {
            let (cols, rows) = crossterm::terminal::size().unwrap();
            (cols as i32, rows as i32)
        };

        let buffer = buffer_manager.active();

        let display_snapshot = buffer.doc.display_map.snapshot();
        let doc_buffer = &buffer.buffer;
        let row_count = display_snapshot.row_count();
        let end_line = (display_snapshot.scroll_y + inner_rect.height as u32).min(row_count);

        let gutter_width = if editor.show_line_numbers {
            2 + if row_count == 0 {
                0
            } else {
                row_count.ilog10() as usize
            }
        } else {
            0
        };

        let theme_fg = editor
            .theme
            .theme
            .settings
            .foreground
            .map(|c| c.rgb())
            .unwrap_or(crossterm::style::Color::White);
        let theme_bg = editor
            .theme
            .theme
            .settings
            .background
            .map(|c| c.rgb())
            .unwrap_or(crossterm::style::Color::Black);
        let theme_caret = editor
            .theme
            .theme
            .settings
            .caret
            .map(|c| c.rgb())
            .unwrap_or(theme_fg);
        let theme_select = editor
            .theme
            .theme
            .settings
            .selection
            .map(|c| c.rgb())
            .unwrap_or(theme_bg);

        let (editor_fg, editor_bg, caret_bg, caret_fg, selection_bg) = if editor.use_colorscheme {
            let fg = editor
                .colorscheme
                .ui
                .get("foreground")
                .map(|s| s.color)
                .unwrap_or(theme_fg);
            let bg = editor
                .colorscheme
                .ui
                .get("background")
                .map(|s| s.color)
                .unwrap_or(theme_bg);
            let sel = editor
                .colorscheme
                .ui
                .get("selection")
                .map(|s| s.color)
                .unwrap_or(theme_select);
            let c_bg = editor
                .colorscheme
                .ui
                .get("caret")
                .map(|s| s.color)
                .unwrap_or(sel);
            let c_fg = fg; // editor.colorscheme.ui.get("caret_foreground").map(|s| s.color).unwrap_or(bg);
            (fg, bg, c_bg, c_fg, sel)
        } else {
            (theme_fg, theme_bg, theme_caret, theme_bg, theme_select)
        };

        let gutter_fg = editor
            .colorscheme
            .ui
            .get("gutter_foreground")
            .map(|s| s.color)
            .unwrap_or(editor_fg);
        let gutter_bg = editor
            .colorscheme
            .ui
            .get("gutter")
            .map(|s| s.color)
            .unwrap_or(editor_bg);
        let find_fg = editor
            .colorscheme
            .ui
            .get("find_highlight_foreground")
            .map(|s| s.color)
            .unwrap_or(editor_fg);
        let find_bg = editor
            .colorscheme
            .ui
            .get("find_highlight")
            .map(|s| s.color)
            .unwrap_or(selection_bg);

        let mut prev_line_number = -1;
        let mut screen_row = inner_rect.y;

        // Scrollbar metrics
        let track_bg = gutter_bg;
        let handle_bg = selection_bg;

        let height = inner_rect.height as u32;
        let handle_h = if row_count > 0 {
            ((height as f32 / row_count as f32) * height as f32)
                .round()
                .max(1.0) as u32
        } else {
            height
        };
        let handle_h = handle_h.min(height);

        let start_y = if row_count > height {
            ((display_snapshot.scroll_y as f32 / (row_count - height) as f32)
                * (height - handle_h) as f32)
                .round() as u32
        } else {
            0
        };

        for row in display_snapshot.scroll_y..end_line {
            {
                execute!(w, MoveTo(inner_rect.x, screen_row)).unwrap();

                // line number
                if editor.show_line_numbers {
                    let line_number = display_snapshot.buffer_row_for_display_row(row);
                    execute!(w, crossterm::style::SetForegroundColor(gutter_fg)).unwrap();

                    execute!(w, crossterm::style::SetBackgroundColor(gutter_bg)).unwrap();
                    if prev_line_number != line_number as i32 {
                        print!("{:>width$} ", (line_number + 1), width = gutter_width - 1);
                    } else {
                        print!("{}", " ".repeat(gutter_width));
                    }
                    prev_line_number = line_number as i32;
                }

                let text = display_snapshot.line_text(row) + " ";

                /*
                let mut matches = Vec::<(usize, usize, &str)>::new();
                if editor.command.pattern {
                    if editor.command.search_text != editor.command.regex_string {
                        editor.command.regex_string = editor.command.search_text.clone();
                        editor.command.regex = compile(editor.command.regex_string.as_str());
                    }
                    if let Some(ref regex) = editor.command.regex {
                        matches = text.as_str().find_pattern(&regex);
                    }
                } else if !editor.command.search_text.is_empty() {
                    matches = text.as_str().find_string(&editor.command.search_text);
                }

                // Convert byte-indexed matches into character-indexed ranges for rendering
                let match_ranges: Vec<(usize, usize)> = matches
                    .iter()
                    .map(|(byte_start, byte_len, _)| {
                        let byte_end = *byte_start + *byte_len;
                        let start_char = text[..*byte_start].chars().count();
                        let end_char = text[..byte_end].chars().count();
                        (start_char, end_char)
                    })
                    .collect();
                let mut match_idx = 0usize;
                */

                let mut x_scroll = display_snapshot.scroll_x;
                let mut cols_remaining = (inner_rect.width as usize).saturating_sub(gutter_width);

                let mut curr_x = inner_rect.x + gutter_width as u16;
                let relative_row = (screen_row - inner_rect.y) as u32;
                let is_handle = relative_row >= start_y && relative_row < start_y + handle_h;

                for (column, ch) in text.chars().enumerate() {
                    let orig_point = display_snapshot
                        .display_point_to_point(DisplayPoint::new(row, column as u32));

                    // Determine if current column is within a search match range
                    let mut in_match = false;

                    /*
                    while match_idx < match_ranges.len() && column >= match_ranges[match_idx].1 {
                        match_idx += 1;
                    }
                    if match_idx < match_ranges.len() {
                        let (s, e) = match_ranges[match_idx];
                        if column >= s && column <= e {
                            in_match = true;
                        }
                    }
                    */

                    let mut fg = editor_fg;
                    let mut bg = editor_bg;

                    if editor.syntax {
                        if let Some(style_cache) = buffer.doc.hl.render_row(orig_point.row) {
                            if let Some(&(style, _, _)) =
                                style_cache.styles.iter().find(|(_, start, end)| {
                                    orig_point.column >= *start && orig_point.column < *end
                                })
                            {
                                fg = style.foreground.rgb();
                                bg = style.background.rgb();
                            }
                        }
                    }

                    // Apply search match background if not in a selection
                    if in_match {
                        fg = find_fg;
                        bg = find_bg;
                    }

                    let (selected, mut selected_line, at_cursor) = buffer
                        .doc
                        .selections()
                        .is_selected(orig_point.row, orig_point.column, &doc_buffer);
                    if selected && (editor.mode != Mode::Command) {
                        bg = selection_bg;
                    }
                    selected_line = selected_line && editor.mode == Mode::VisualLine;
                    if selected_line {
                        bg = selection_bg;
                    }

                    if at_cursor {
                        bg = selection_bg;
                        // bg = caret_bg;
                        // fg = caret_fg;
                    }

                    if x_scroll > 0 {
                        x_scroll = x_scroll.saturating_sub(1);
                    } else {
                        let is_scrollbar = curr_x == inner_rect.x + inner_rect.width - 1;
                        let bg_color = if is_scrollbar {
                            if is_handle { handle_bg } else { track_bg }
                        } else {
                            bg
                        };

                        execute!(w, crossterm::style::SetForegroundColor(fg)).unwrap();
                        execute!(w, crossterm::style::SetBackgroundColor(bg_color)).unwrap();

                        match ch {
                            '\t' => {
                                for _i in 0..4 {
                                    // Tab size of 4
                                    let is_scrollbar_tab =
                                        curr_x == inner_rect.x + inner_rect.width - 1;
                                    let cell_bg = if is_scrollbar_tab {
                                        if is_handle { handle_bg } else { track_bg }
                                    } else if at_cursor
                                        && editor.mode != Mode::Insert
                                        && editor.mode != Mode::Command
                                    {
                                        editor_bg
                                    } else {
                                        bg
                                    };
                                    execute!(w, crossterm::style::SetBackgroundColor(cell_bg))
                                        .unwrap();
                                    print!(" ");
                                    curr_x += 1;
                                    cols_remaining = cols_remaining.saturating_sub(1);
                                }
                            }
                            _ => {
                                print!("{}", ch);
                                curr_x += 1;
                                cols_remaining = cols_remaining.saturating_sub(1);
                            }
                        }
                    }

                    if cols_remaining <= 0 {
                        break;
                    }
                }

                for _ in 0..cols_remaining {
                    let is_scrollbar = curr_x == inner_rect.x + inner_rect.width - 1;
                    let bg_color = if is_scrollbar {
                        if is_handle { handle_bg } else { track_bg }
                    } else {
                        editor_bg
                    };
                    execute!(w, crossterm::style::SetBackgroundColor(bg_color)).unwrap();
                    print!(" ");
                    curr_x += 1;
                }

                screen_row += 1;
                if screen_row >= inner_rect.y + inner_rect.height {
                    break;
                }
            }
        }

        Ok(())
    }
}

impl View for TextView {
    fn draw(
        &self,
        mut w: &mut dyn Write,
        rect: Rect,
        editor: &Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.draw_textview(&mut w, rect, editor, buffer_manager)
    }
}
