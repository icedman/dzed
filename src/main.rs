mod actions;
mod background;
mod display;
mod document;
mod editor;
mod highlight;
mod input;
mod profiler;
mod search;
mod selections;
mod theme;

use crate::profiler::Profiler;
use crate::search::{TextSearch, compile};
use crate::theme::{ColorAdjust, ToCrossTerm};

use std::{
    io::{Write, stdout},
    time::{Duration, Instant},
};

use crossterm::{
    cursor::MoveTo,
    event::{self},
    execute,
    terminal::{Clear, ClearType},
};

use text::ToPoint;

use actions::Mode;
use document::BufferText;
use editor::Editor;
use input::{HandleEvent, handle_event};

fn fill_to_eol(count: usize) {
    for _ in 0..count {
        print!(" ");
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut profiler = Profiler::new();

    let args: Vec<String> = std::env::args().collect();
    let file_paths = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        Vec::new()
    };

    let mut editor = Editor::new(file_paths)?;
    let mut stdout = stdout();
    crossterm::terminal::enable_raw_mode().unwrap();
    execute!(
        stdout,
        crossterm::event::EnableBracketedPaste,
        Clear(ClearType::All),
        MoveTo(0, 0)
    )
    .unwrap();

    execute!(
        stdout,
        crossterm::event::EnableMouseCapture,
        Clear(ClearType::All),
        MoveTo(0, 0)
    )?;

    let tab_size = 4;
    execute!(stdout, crossterm::cursor::Hide).unwrap();

    let mut should_redraw = true;
    let mut should_sync = true;
    let mut prev_screen_rows = 0;
    let mut prev_screen_cols = 0;
    let mut last_cursor_style = None;

    let mut ticks: Duration = Duration::ZERO;

    loop {
        let start = Instant::now();

        // get screen dimensions
        let (screen_cols, screen_rows) = {
            let (cols, rows) = crossterm::terminal::size().unwrap();
            (cols as i32, rows as i32)
        };
        // dimensions has changed
        if prev_screen_cols != screen_cols || prev_screen_rows != screen_rows {
            should_redraw = true;
        }
        prev_screen_rows = screen_rows;
        prev_screen_cols = screen_cols;

        // Drain any incoming background worker results
        while let Some(result) = editor.bg_worker.try_recv() {
            match result {
                background::BackgroundResult::HighlightComplete {
                    file_path,
                    style_cache,
                    task_id,
                } => {
                    if let Some(buf) = editor
                        .buffer_manager
                        .buffers
                        .iter_mut()
                        .find(|b| b.file_path == file_path)
                    {
                        if task_id >= background::TaskId(buf.current_hl_task_id) {
                            buf.current_hl_task_id = task_id.0;
                            buf.hl
                                .merge_caches(style_cache, std::collections::HashMap::new());
                            should_redraw = true;
                        }
                    }
                }
                background::BackgroundResult::WrapComplete {
                    file_path,
                    wrap_snapshot,
                    task_id,
                } => {
                    if let Some(buf) = editor
                        .buffer_manager
                        .buffers
                        .iter_mut()
                        .find(|b| b.file_path == file_path)
                    {
                        if task_id >= background::TaskId(buf.current_wrap_task_id) {
                            buf.current_wrap_task_id = task_id.0;
                            buf.display_map.apply_wrap_snapshot(wrap_snapshot);
                            should_redraw = true;
                        }
                    }
                }
            }
        }

        let active_buffer = editor.buffer_manager.active_mut();
        editor.mode = active_buffer.doc.current_mode();

        // Update layout before wrapping so the wrap width reflects the current gutter.
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

        active_buffer.display_map.margin_left = gutter_width as u32;
        let wrap_cols = screen_cols
            .saturating_sub(active_buffer.display_map.margin_left as i32)
            .saturating_sub(active_buffer.display_map.margin_right as i32)
            .max(1);
        active_buffer
            .display_map
            .set_wrap_width(editor.wrap.then_some(wrap_cols as u32));

        if should_sync {
            profiler.profile("display_map.sync", || {
                active_buffer
                    .display_map
                    .sync(active_buffer.doc.buffer().snapshot().clone());
            });
            active_buffer.dirty_hl = true;

            let (start, _) = active_buffer
                .doc
                .selections()
                .rows_in_selection(active_buffer.doc.buffer());
            active_buffer.hl.invalidate_state(start);

            // Spawn background highlight task
            let hl_task_id = active_buffer
                .latest_hl_task_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            editor
                .bg_worker
                .spawn_task(background::BackgroundTask::Highlight {
                    file_path: active_buffer.file_path.clone(),
                    snapshot: active_buffer.doc.buffer().snapshot().clone(),
                    start_row: start,
                    row_count: active_buffer.doc.buffer().row_count() - start,
                    theme: std::sync::Arc::new(editor.theme.theme.clone()),
                    task_id: background::TaskId(hl_task_id),
                    latest_task_id: active_buffer.latest_hl_task_id.clone(),
                });

            // Spawn background wrap task
            let wrap_width = editor.wrap.then_some(wrap_cols as u32);
            let wrap_task_id = active_buffer
                .latest_wrap_task_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            editor
                .bg_worker
                .spawn_task(background::BackgroundTask::Wrap {
                    file_path: active_buffer.file_path.clone(),
                    snapshot: active_buffer.doc.buffer().snapshot().clone(),
                    wrap_width,
                    task_id: background::TaskId(wrap_task_id),
                    latest_task_id: active_buffer.latest_wrap_task_id.clone(),
                });

            should_sync = false;
        }

        let cursor = active_buffer.doc.selection();
        let cursor_point = cursor.head().to_point(active_buffer.doc.buffer());
        let display_cursor = active_buffer
            .display_map
            .snapshot()
            .point_to_display_point(cursor_point);
        active_buffer
            .display_map
            .scroll_to_cursor(display_cursor, screen_rows, screen_cols);

        let display_snapshot = active_buffer.display_map.snapshot();
        let cursor_row = display_cursor.row() as i32;
        let cursor_col = display_cursor.column() as i32;

        let visible_rows = (screen_rows - 1)
            .saturating_sub(display_snapshot.margin_top as i32)
            .saturating_sub(display_snapshot.margin_bottom as i32);

        // scroll based on cursor position
        let cursor_screen_row = cursor_row - display_snapshot.scroll_y as i32;
        let cursor_screen_col = cursor_col - display_snapshot.scroll_x as i32;

        //------------------
        // render
        //------------------
        if should_redraw {
            should_redraw = false;
            execute!(stdout, crossterm::cursor::Hide).unwrap();

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
                    profiler.profile("hl.highlight_lines", || {
                        active_buffer.hl.highlight_lines(
                            &active_buffer.doc.buffer().snapshot(),
                            start_buffer_row,
                            end_buffer_row_exclusive - start_buffer_row,
                            &editor.theme.theme,
                        );
                    });
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
                        crossterm::style::SetBackgroundColor(editor.theme.gutter) // crossterm::style::SetBackgroundColor(
                                                                                  //     default_style.background.darken(10).rgb()
                                                                                  // )
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
                profiler.profile("search", || {
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
                });
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
                    if at_cursor && editor.mode != Mode::Insert && editor.mode != Mode::Command {
                        // let cursor blink for us
                        // fg = editor.theme.bg;
                        // bg = editor.theme.caret;
                    }

                    execute!(stdout, crossterm::style::SetForegroundColor(fg)).unwrap();
                    execute!(stdout, crossterm::style::SetBackgroundColor(bg)).unwrap();

                    if x_scroll > 0 {
                        x_scroll = x_scroll.saturating_sub(1);
                    } else {
                        match ch {
                            '\t' => {
                                for _i in 0..tab_size {
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

            // statusbar
            {
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

                if editor.mode == Mode::Command {
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
                } else {
                    let active_idx = editor.buffer_manager.active_idx;
                    let buffer_count = editor.buffer_manager.buffers.len();
                    let active_buffer = editor.buffer_manager.active();
                    let row_len = active_buffer.doc.buffer().line_len(cursor_point.row as u32);

                    print!(
                        "[{}/{}] {} {} {},{} rl:{} {} {} [{}]",
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
                        editor.search_text
                    );
                }
            }

            let needed_style = if editor.mode == Mode::Command {
                crossterm::cursor::SetCursorStyle::BlinkingBar
            } else {
                match editor.mode {
                    Mode::Insert => crossterm::cursor::SetCursorStyle::BlinkingBar,
                    _ => crossterm::cursor::SetCursorStyle::BlinkingBlock,
                }
            };

            if last_cursor_style != Some(needed_style) {
                execute!(stdout, needed_style).unwrap();
                last_cursor_style = Some(needed_style);
            }

            if editor.mode == Mode::Command {
                let cmd_text = editor
                    .cmd
                    .buffer()
                    .row_text(editor.cmd.buffer().row_count() - 1);
                let cmd_col = (cmd_text.chars().count()) as u16;
                execute!(
                    stdout,
                    MoveTo(cmd_col + 1, screen_rows as u16),
                    // crossterm::cursor::Show
                )
                .unwrap();
            } else {
                execute!(
                    stdout,
                    MoveTo(
                        display_snapshot.margin_left as u16 + cursor_screen_col as u16,
                        display_snapshot.margin_top as u16 + cursor_screen_row as u16
                    ),
                    // crossterm::cursor::Show
                )
                .unwrap();
            }

            if editor.mode == Mode::Insert {
                execute!(stdout, crossterm::cursor::Show).unwrap();
                ticks = Duration::ZERO;
            }

            stdout.flush().unwrap();
        }

        let elapsed = start.elapsed();
        ticks += elapsed;

        //------------------
        // input
        //------------------
        if event::poll(Duration::from_millis(50))? {
            let event = event::read()?;
            match handle_event(&mut editor, event, visible_rows) {
                HandleEvent::Exit => break,
                HandleEvent::Redraw => should_redraw = true,
                HandleEvent::RedrawAndSync => {
                    should_redraw = true;
                    should_sync = true;
                }
                HandleEvent::NoRedraw => {}
            }
        }
    }

    crossterm::terminal::disable_raw_mode().unwrap();
    execute!(
        stdout,
        crossterm::event::DisableBracketedPaste,
        Clear(ClearType::All),
        MoveTo(0, 0)
    )
    .unwrap();
    execute!(
        stdout,
        crossterm::event::DisableMouseCapture,
        Clear(ClearType::All),
        MoveTo(0, 0)
    )
    .unwrap();

    Ok(())
}
