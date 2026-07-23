use super::layout::Rect;
use super::view::View;
use crate::actions::Mode;
use crate::document::BufferText;
use crate::editor::Editor;
use crate::search::{TextSearch, compile};
use crate::theme::{ColorAdjust, ToCrossTerm};
use crossterm::{cursor::MoveTo, execute};
use std::io::Write;
use text::ToPoint;

/// A standard view that renders the active text editor buffer.
pub struct EditorView;

impl EditorView {
    pub fn new() -> Self {
        EditorView
    }
}

impl View for EditorView {
    fn draw(
        &mut self,
        mut w: &mut dyn Write,
        rect: Rect,
        editor: &mut Editor,
    ) -> std::io::Result<()> {
        let active_buffer = editor.buffer_manager.active_mut();
        let row_count = active_buffer.doc.buffer().row_count();
        let gutter_width = if editor.show_line_numbers {
            2 + if row_count == 0 {
                0
            } else {
                row_count.ilog10() as usize
            }
        } else {
            0
        };

        render_editor_content(&mut w, editor, gutter_width, rect)?;
        Ok(())
    }

    fn handle_event(
        &mut self,
        event: &crossterm::event::Event,
        editor: &mut Editor,
    ) -> Option<crate::input::HandleEvent> {
        let active_buffer = editor.buffer_manager.active();
        let display_snapshot = active_buffer.display_map.snapshot();
        let visible_rows = display_snapshot.row_count() as i32;

        Some(crate::input::handle_event(
            editor,
            event.clone(),
            visible_rows,
        ))
    }
}

fn fill_to_eol(count: usize) {
    for _ in 0..count {
        print!(" ");
    }
}

/// Renders the core editor buffer lines including line numbers, highlight groups, and active selections.
pub fn render_editor_content<W: Write>(
    stdout: &mut W,
    editor: &mut Editor,
    gutter_width: usize,
    inner_rect: Rect,
) -> std::io::Result<()> {
    let active_buffer = editor.buffer_manager.active_mut();
    let display_snapshot = active_buffer.display_map.snapshot();
    let buffer = active_buffer.doc.buffer();
    let total_rows = display_snapshot.row_count();
    let end_line = (display_snapshot.scroll_y + inner_rect.height as u32).min(total_rows);

    if editor.syntax && end_line > display_snapshot.scroll_y {
        let start_buffer_row =
            display_snapshot.buffer_row_for_display_row(display_snapshot.scroll_y);
        let end_buffer_row =
            display_snapshot.buffer_row_for_display_row(end_line.saturating_sub(1));
        let end_buffer_row_exclusive = end_buffer_row + 1;

        if active_buffer.dirty_hl
            || !active_buffer
                .hl
                .contains_rows(start_buffer_row, end_buffer_row_exclusive)
        {
            active_buffer.hl.highlight_lines(
                &active_buffer.doc.buffer().snapshot(),
                start_buffer_row,
                end_buffer_row_exclusive - start_buffer_row,
                &editor.theme.theme,
            );
            active_buffer.dirty_hl = false;
        }
    }

    let mut prev_line_number = -1;
    let mut screen_row = inner_rect.y;

    for row in display_snapshot.scroll_y..end_line {
        execute!(stdout, MoveTo(inner_rect.x, screen_row)).unwrap();

        // line number
        if editor.show_line_numbers {
            let line_number = display_snapshot.buffer_row_for_display_row(row);
            execute!(
                stdout,
                crossterm::style::SetForegroundColor(editor.theme.gutter_fg)
            )
            .unwrap();

            execute!(
                stdout,
                crossterm::style::SetBackgroundColor(editor.theme.gutter)
            )
            .unwrap();
            if prev_line_number != line_number as i32 {
                print!("{:>width$} ", (line_number + 1), width = gutter_width - 1);
            } else {
                print!("{}", " ".repeat(gutter_width));
            }
            prev_line_number = line_number as i32;
        }

        let text = display_snapshot.line_text(row) + " ";

        let mut matches = Vec::<(usize, usize, &str)>::new();
        if editor.pattern {
            if editor.search_text != editor.regex_string {
                editor.regex_string = editor.search_text.clone();
                editor.regex = compile(editor.regex_string.as_str());
            }
            if let Some(ref regex) = editor.regex {
                matches = text.as_str().find_pattern(&regex);
            }
        } else if !editor.search_text.is_empty() {
            matches = text.as_str().find_string(&editor.search_text);
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

        let buffer_row = display_snapshot.buffer_row_for_display_row(row);
        let buffer_range = display_snapshot.buffer_range_for_display_row(row);
        let start_col = buffer_range.start.column;

        let ranges = if editor.syntax {
            active_buffer
                .hl
                .render_row(buffer_row)
                .map(|style_cache| style_cache.styles.as_slice())
                .unwrap_or(&[])
        } else {
            &[]
        };

        let mut range_idx = ranges.partition_point(|(_, _, end)| *end <= start_col);

        let mut x_scroll = display_snapshot.scroll_x;
        let mut cols_remaining = (inner_rect.width as usize).saturating_sub(gutter_width);

        let mut byte_column = start_col;
        for (column, ch) in text.chars().enumerate() {
            let rc = byte_column;
            // Determine if current column is within a search match range
            let mut in_match = false;
            while match_idx < match_ranges.len() && column >= match_ranges[match_idx].1 {
                match_idx += 1;
            }
            if match_idx < match_ranges.len() {
                let (s, e) = match_ranges[match_idx];
                if column >= s && column <= e {
                    in_match = true;
                }
            }

            while range_idx < ranges.len() && ranges[range_idx].2 <= rc {
                range_idx += 1;
            }

            let mut fg = editor.theme.fg;
            let mut bg = editor.theme.bg;

            if editor.syntax
                && let Some((style, start, end)) = ranges.get(range_idx)
                && *start <= rc
                && rc < *end
            {
                fg = style.foreground.rgb();
                bg = style.background.darken(10).rgb();
            }
            // Apply search match background if not in a selection
            if in_match {
                fg = editor.theme.find_fg;
                bg = editor.theme.find;
            }

            let (selected, mut selected_line, at_cursor) = active_buffer
                .doc
                .selections()
                .is_selected(buffer_row, rc, &buffer);
            if selected && (editor.mode != Mode::Command) {
                bg = editor.theme.select;
            }
            selected_line = selected_line && editor.mode == Mode::VisualLine;
            if selected_line {
                bg = editor.theme.select;
            }

            execute!(stdout, crossterm::style::SetForegroundColor(fg)).unwrap();
            execute!(stdout, crossterm::style::SetBackgroundColor(bg)).unwrap();

            if x_scroll > 0 {
                x_scroll = x_scroll.saturating_sub(1);
            } else {
                match ch {
                    '\t' => {
                        for _i in 0..4 {
                            // Tab size of 4
                            print!(" ");
                            if at_cursor
                                && editor.mode != Mode::Insert
                                && editor.mode != Mode::Command
                            {
                                execute!(
                                    stdout,
                                    crossterm::style::SetBackgroundColor(editor.theme.bg)
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

            byte_column += ch.len_utf8() as u32;

            if cols_remaining <= 0 {
                break;
            }
        }

        execute!(
            stdout,
            crossterm::style::SetBackgroundColor(editor.theme.bg)
        )
        .unwrap();
        fill_to_eol(cols_remaining);

        screen_row += 1;
        if screen_row >= inner_rect.y + inner_rect.height {
            break;
        }
    }
    Ok(())
}

/// Positions and styles the hardware terminal cursor.
pub fn update_cursor_position<W: Write>(
    stdout: &mut W,
    editor: &Editor,
    display_snapshot: &crate::display::display_map::DisplaySnapshot,
    inner_rect: Rect,
    gutter_width: usize,
    cursor_screen_col: i32,
    cursor_screen_row: i32,
    last_cursor_style: &mut Option<crossterm::cursor::SetCursorStyle>,
) -> std::io::Result<()> {
    let needed_style = if editor.mode == Mode::Command {
        crossterm::cursor::SetCursorStyle::BlinkingBar
    } else {
        match editor.mode {
            Mode::Insert => crossterm::cursor::SetCursorStyle::BlinkingBar,
            _ => crossterm::cursor::SetCursorStyle::BlinkingBlock,
        }
    };

    if last_cursor_style != &mut Some(needed_style) {
        execute!(stdout, needed_style).unwrap();
        *last_cursor_style = Some(needed_style);
    }

    if editor.mode == Mode::Command {
        let cmd_text = editor
            .cmd
            .buffer()
            .row_text(editor.cmd.buffer().row_count() - 1);
        let cmd_col = (cmd_text.chars().count()) as u16;
        // Command line is drawn at statusbar rect y position
        execute!(
            stdout,
            MoveTo(cmd_col + 1, inner_rect.y + inner_rect.height + 1),
        )
        .unwrap();
    } else {
        execute!(
            stdout,
            MoveTo(
                inner_rect.x + gutter_width as u16 + cursor_screen_col as u16,
                inner_rect.y + cursor_screen_row as u16
            ),
        )
        .unwrap();
    }

    execute!(stdout, crossterm::cursor::Show).unwrap();
    Ok(())
}
