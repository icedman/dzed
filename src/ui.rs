use crate::actions::Mode;
use crate::document::{BufferText, Document};
use crate::editor::Editor;
use crate::search::{TextSearch, compile};
use crate::theme::{ColorAdjust, ToCrossTerm};
use crossterm::{cursor::MoveTo, execute};
use std::io::{Stdout, Write};
use text::{Point, ToPoint};

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
    screen_rows: i32,
    screen_cols: i32,
    visible_rows: i32,
) -> std::io::Result<()> {
    let active_buffer = editor.buffer_manager.active_mut();
    let display_snapshot = active_buffer.display_map.snapshot();
    let buffer = active_buffer.doc.buffer();
    let total_rows = display_snapshot.row_count();
    let end_line = (display_snapshot.scroll_y + visible_rows as u32).min(total_rows);

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
    let mut screen_row = display_snapshot.margin_top as u16;

    for row in display_snapshot.scroll_y..end_line {
        execute!(
            stdout,
            MoveTo(
                (display_snapshot.x() - gutter_width as u32) as u16,
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
        let mut cols_remaining = screen_cols
            .saturating_sub(display_snapshot.margin_left as i32)
            .saturating_sub(display_snapshot.margin_right as i32);

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
        fill_to_eol(cols_remaining.max(0) as usize);

        screen_row += 1;
        if screen_row + 1 > screen_rows as u16 {
            break;
        }
    }
    Ok(())
}

/// Renders the standard info status bar at the bottom row.
pub fn render_status_bar(
    stdout: &mut Stdout,
    editor: &Editor,
    cursor_point: Point,
    screen_rows: i32,
    screen_cols: i32,
) -> std::io::Result<()> {
    execute!(
        stdout,
        crossterm::style::SetForegroundColor(editor.theme.fg)
    )
    .unwrap();
    execute!(
        stdout,
        crossterm::style::SetBackgroundColor(editor.theme.gutter)
    )
    .unwrap();
    execute!(stdout, MoveTo(0, screen_rows as u16)).unwrap();
    fill_to_eol(screen_cols as usize);
    execute!(stdout, MoveTo(0, screen_rows as u16)).unwrap();

    let active_idx = editor.buffer_manager.active_idx;
    let buffer_count = editor.buffer_manager.buffers.len();
    let active_buffer = editor.buffer_manager.active();
    let buffer = active_buffer.doc.buffer();
    let row_len = buffer.line_len(cursor_point.row as u32);
    let selection = active_buffer.doc.selection();
    let cursor_offset = buffer.offset_for_anchor(&selection.head());
    let syntax_context = editor
        .tree_sitter
        .then_some(active_buffer.syntax_tree.as_ref())
        .flatten()
        .map(|syntax_tree| {
            let node = syntax_tree
                .named_node_at_byte(cursor_offset)
                .map(|node| node.kind)
                .unwrap_or_else(|| "?".to_string());
            let scope = syntax_tree
                .current_scope(buffer.snapshot(), cursor_offset)
                .map(|scope| scope.name.unwrap_or(scope.kind))
                .unwrap_or_else(|| "-".to_string());
            format!(
                "ts:{} node:{node} scope:{scope}",
                syntax_tree.grammar().name()
            )
        })
        .unwrap_or_else(|| {
            if editor.tree_sitter {
                "ts:- node:- scope:-".to_string()
            } else {
                "ts:off".to_string()
            }
        });

    print!(
        "[{}/{}] {} {} {},{} rl:{} {} {} [{}] {}",
        active_idx + 1,
        buffer_count,
        active_buffer.file_path,
        match editor.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Visual => "VISUAL",
            Mode::VisualLine => "V-LINE",
            Mode::VisualBlock => "V-BLOCK",
            Mode::Command => "COMMAND",
        },
        active_buffer.doc.selection().head().offset,
        active_buffer.doc.selection().tail().offset,
        row_len,
        active_buffer.hl.name(),
        editor.pending_cmd,
        editor.search_text,
        syntax_context
    );
    Ok(())
}

/// Renders the active command or search input prompt at the bottom row.
pub fn render_command_line(
    stdout: &mut Stdout,
    editor: &Editor,
    screen_rows: i32,
    screen_cols: i32,
) -> std::io::Result<()> {
    execute!(
        stdout,
        crossterm::style::SetForegroundColor(editor.theme.fg)
    )
    .unwrap();
    execute!(
        stdout,
        crossterm::style::SetBackgroundColor(editor.theme.gutter)
    )
    .unwrap();
    execute!(stdout, MoveTo(0, screen_rows as u16)).unwrap();
    fill_to_eol(screen_cols as usize);
    execute!(stdout, MoveTo(0, screen_rows as u16)).unwrap();

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
    screen_rows: i32,
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
        execute!(stdout, MoveTo(cmd_col + 1, screen_rows as u16),).unwrap();
    } else {
        execute!(
            stdout,
            MoveTo(
                display_snapshot.margin_left as u16 + cursor_screen_col as u16,
                display_snapshot.margin_top as u16 + cursor_screen_row as u16
            ),
        )
        .unwrap();
    }

    if editor.mode == Mode::Insert {
        execute!(stdout, crossterm::cursor::Show).unwrap();
    }
    Ok(())
}

/// Coordinates the complete rendering workflow by delegating to specialized renderers.
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
    execute!(stdout, crossterm::cursor::Hide).unwrap();

    // 1. Render actual buffer content rows
    render_editor_content(
        stdout,
        editor,
        gutter_width,
        screen_rows,
        screen_cols,
        visible_rows,
    )?;

    // 2. Render status bar or command line depending on mode
    if editor.mode == Mode::Command {
        render_command_line(stdout, editor, screen_rows, screen_cols)?;
    } else {
        let active_buffer = editor.buffer_manager.active();
        let cursor_point = active_buffer
            .doc
            .selection()
            .head()
            .to_point(active_buffer.doc.buffer());
        render_status_bar(stdout, editor, cursor_point, screen_rows, screen_cols)?;
    }

    // 3. Update cursor position
    let active_buffer = editor.buffer_manager.active();
    let display_snapshot = active_buffer.display_map.snapshot();
    update_cursor_position(
        stdout,
        editor,
        &display_snapshot,
        screen_rows,
        cursor_screen_col,
        cursor_screen_row,
        last_cursor_style,
    )?;

    stdout.flush().unwrap();
    Ok(())
}
