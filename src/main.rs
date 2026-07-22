mod actions;
mod background;
mod clipboard;
mod display;
mod document;
mod editor;
mod highlight;
mod input;
mod keymap;
mod search;
mod selections;
mod theme;
mod treesitter;
mod ui;


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
use editor::Editor;
use input::{HandleEvent, handle_event};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
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
                background::BackgroundResult::ParseComplete {
                    file_path,
                    syntax_tree,
                    task_id,
                } => {
                    if editor.tree_sitter
                        && let Some(buf) = editor
                            .buffer_manager
                            .buffers
                            .iter_mut()
                            .find(|b| b.file_path == file_path)
                        && task_id >= background::TaskId(buf.current_parse_task_id)
                    {
                        buf.current_parse_task_id = task_id.0;
                        buf.syntax_tree = Some(syntax_tree);
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
            active_buffer
                .display_map
                .sync(active_buffer.doc.buffer().snapshot().clone());
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

            if editor.tree_sitter
                && let Some(grammar) = active_buffer.grammar
            {
                let parse_task_id = active_buffer
                    .latest_parse_task_id
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                    + 1;
                editor
                    .bg_worker
                    .spawn_task(background::BackgroundTask::Parse {
                        file_path: active_buffer.file_path.clone(),
                        snapshot: active_buffer.doc.buffer().snapshot().clone(),
                        grammar,
                        task_id: background::TaskId(parse_task_id),
                        latest_task_id: active_buffer.latest_parse_task_id.clone(),
                    });
            }

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
            ui::render(
                &mut stdout,
                &mut editor,
                gutter_width,
                screen_rows,
                screen_cols,
                visible_rows,
                cursor_screen_row,
                cursor_screen_col,
                &mut last_cursor_style,
            )?;
            if editor.mode == Mode::Insert {
                ticks = Duration::ZERO;
            }
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
