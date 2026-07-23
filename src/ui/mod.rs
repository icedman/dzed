pub mod layout;
pub mod window;
pub mod tabs;
pub mod statusbar;

use crate::actions::Mode;
use crate::document::BufferText;
use crate::editor::Editor;
use crate::search::{TextSearch, compile};
use crate::theme::{ColorAdjust, ToCrossTerm};
use crossterm::{cursor::MoveTo, execute};
use std::io::{Stdout, Write};
use text::{Point, ToPoint};

/// The main UI class managing layouts, windows, and focus state.
pub struct Ui {
    pub layout: layout::LayoutNode,
    pub windows: std::collections::HashMap<usize, window::Window>,
    pub focused_window_id: Option<usize>,
}

impl Ui {
    pub fn new() -> Self {
        let mut windows = std::collections::HashMap::new();
        // Create initial default window
        let main_win_id = 0;
        windows.insert(main_win_id, window::Window::new(main_win_id, "Editor".to_string()));

        let layout = layout::LayoutNode::Split {
            direction: layout::SplitDirection::Vertical,
            constraints: vec![
                layout::SizeConstraint::Fixed(1),        // Tabs (1 row)
                layout::SizeConstraint::Percentage(1.0), // Editor
                layout::SizeConstraint::Fixed(1),        // Statusbar (1 row)
            ],
            children: vec![
                layout::LayoutNode::Leaf { window_id: 1 }, // Tabs
                layout::LayoutNode::Leaf { window_id: 0 }, // Editor
                layout::LayoutNode::Leaf { window_id: 2 }, // Statusbar
            ],
        };

        Self {
            layout,
            windows,
            focused_window_id: Some(main_win_id),
        }
    }

    /// Renders the layout and all windows/components managed by this UI instance.
    pub fn draw(
        &mut self,
        stdout: &mut Stdout,
        editor: &mut Editor,
        screen_width: u16,
        screen_height: u16,
        last_cursor_style: &mut Option<crossterm::cursor::SetCursorStyle>,
    ) -> std::io::Result<()> {
        execute!(stdout, crossterm::cursor::Hide)?;

        let parent_rect = layout::Rect {
            x: 0,
            y: 0,
            width: screen_width,
            height: screen_height,
        };
        let computed = self.layout.compute_layout(parent_rect);
        
        // Find editor window inner rect to position text & cursor correctly
        let mut editor_inner_rect = parent_rect;
        let mut editor_gutter_width = 0usize;

        for (win_id, rect) in &computed {
            if *win_id == 0 {
                editor_inner_rect = layout::Rect {
                    x: rect.x + 1,
                    y: rect.y + 1,
                    width: rect.width.saturating_sub(2),
                    height: rect.height.saturating_sub(2),
                };

                let active_buffer = editor.buffer_manager.active_mut();
                let row_count = active_buffer.doc.buffer().row_count();
                editor_gutter_width = if editor.show_line_numbers {
                    2 + if row_count == 0 {
                        0
                    } else {
                        row_count.ilog10() as usize
                    }
                } else {
                    0
                };
                break;
            }
        }

        for (win_id, rect) in computed {
            match win_id {
                0 => {
                    // 1. Draw Editor window border
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.is_focused = Some(win_id) == self.focused_window_id;
                        win.draw(stdout, rect)?;
                    }
                    // 2. Draw Editor content inside border
                    render_editor_content(stdout, editor, editor_gutter_width, editor_inner_rect)?;
                }
                1 => {
                    let tabs = tabs::Tabs::new();
                    tabs.draw(stdout, rect, editor)?;
                }
                2 => {
                    if editor.mode == Mode::Command {
                        render_command_line(stdout, editor, rect)?;
                    } else {
                        let statusbar = statusbar::StatusBar::new();
                        let active_buffer = editor.buffer_manager.active();
                        let cursor_point = active_buffer
                            .doc
                            .selection()
                            .head()
                            .to_point(active_buffer.doc.buffer());
                        statusbar.draw(stdout, rect, editor, cursor_point)?;
                    }
                }
                _ => {}
            }
        }

        // 3. Update cursor position
        let active_buffer = editor.buffer_manager.active();
        let display_snapshot = active_buffer.display_map.snapshot();
        
        let cursor = active_buffer.doc.selection();
        let cursor_point = cursor.head().to_point(active_buffer.doc.buffer());
        let display_cursor = active_buffer
            .display_map
            .snapshot()
            .point_to_display_point(cursor_point);
            
        let cursor_row = display_cursor.row() as i32;
        let cursor_col = display_cursor.column() as i32;
        let cursor_screen_row = cursor_row - display_snapshot.scroll_y as i32;
        let cursor_screen_col = cursor_col - display_snapshot.scroll_x as i32;

        update_cursor_position(
            stdout,
            editor,
            &display_snapshot,
            editor_inner_rect,
            editor_gutter_width,
            cursor_screen_col,
            cursor_screen_row,
            last_cursor_style,
        )?;

        stdout.flush()?;
        Ok(())
    }
}

fn fill_to_eol(count: usize) {
    for _ in 0..count {
        print!(" ");
    }
}

/// Renders the core editor buffer lines including line numbers, highlight groups, and active selections.
pub fn render_editor_content(
    stdout: &mut Stdout,
    editor: &mut Editor,
    gutter_width: usize,
    inner_rect: layout::Rect,
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
        execute!(
            stdout,
            MoveTo(
                inner_rect.x,
                screen_row
            )
        )
        .unwrap();

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

/// Renders the standard info status bar at the bottom row (Delegates to StatusBar component).
pub fn render_status_bar(
    stdout: &mut Stdout,
    editor: &Editor,
    cursor_point: Point,
    screen_rows: i32,
    screen_cols: i32,
) -> std::io::Result<()> {
    let status_rect = layout::Rect {
        x: 0,
        y: screen_rows as u16,
        width: screen_cols as u16,
        height: 1,
    };
    statusbar::StatusBar::new().draw(stdout, status_rect, editor, cursor_point)
}

/// Renders the active command or search input prompt at the bottom row.
pub fn render_command_line(
    stdout: &mut Stdout,
    editor: &Editor,
    rect: layout::Rect,
) -> std::io::Result<()> {
    execute!(
        stdout,
        crossterm::style::SetForegroundColor(editor.theme.fg),
        crossterm::style::SetBackgroundColor(editor.theme.gutter),
        MoveTo(rect.x, rect.y)
    )?;
    fill_to_eol(rect.width as usize);
    execute!(stdout, MoveTo(rect.x, rect.y))?;

    let mut cmd_char = ':';
    if editor.search {
        cmd_char = '/';
        if editor.pattern {
            cmd_char = '?';
        }
    }
    print!(
        "{}{}",
        cmd_char,
        editor
            .cmd
            .buffer()
            .row_text(editor.cmd.buffer().row_count() - 1)
    );
    Ok(())
}

/// Positions and styles the hardware terminal cursor.
pub fn update_cursor_position(
    stdout: &mut Stdout,
    editor: &Editor,
    display_snapshot: &crate::display::display_map::DisplaySnapshot,
    inner_rect: layout::Rect,
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
        execute!(stdout, MoveTo(cmd_col + 1, inner_rect.y + inner_rect.height + 1),).unwrap();
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

/// Legacy coordinates rendering method (delegated to Ui now)
pub fn render(
    stdout: &mut Stdout,
    editor: &mut Editor,
    gutter_width: usize,
    screen_rows: i32,
    screen_cols: i32,
    visible_rows: i32,
    cursor_screen_row: i32,
    cursor_screen_col: i32,
    last_cursor_style: &mut Option<crossterm::cursor::SetCursorStyle>,
) -> std::io::Result<()> {
    let mut ui = Ui::new();
    ui.draw(stdout, editor, screen_cols as u16, screen_rows as u16, last_cursor_style)
}
