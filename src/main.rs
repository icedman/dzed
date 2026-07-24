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

    let mut ui = ui::Ui::new();
    let mut should_redraw = true;
    let mut should_sync = true;
    let mut prev_screen_rows = 0;
    let mut prev_screen_cols = 0;
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

        let parent_rect = ui::layout::Rect {
            x: 0,
            y: 0,
            width: screen_cols as u16,
            height: screen_rows as u16,
        };
        if ui.dirty || ui.last_parent_rect != Some(parent_rect) {
            ui.cached_layouts = ui.layout.compute_layout(parent_rect);
            ui.last_parent_rect = Some(parent_rect);
            ui.dirty = false;
        }

        ui.update(&mut editor, &mut should_sync)?;

        let editor_rect = ui
            .cached_layouts
            .iter()
            .find(|(id, _)| *id == 0)
            .map(|(_, rect)| *rect)
            .unwrap_or(parent_rect);
        let editor_inner_height = editor_rect.height.saturating_sub(2);
        let visible_rows = editor_inner_height as i32;

        //------------------
        // render
        //------------------
        if should_redraw {
            should_redraw = false;
            ui.draw(
                &mut stdout,
                &mut editor,
                screen_cols as u16,
                screen_rows as u16,
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
            let event_res = ui
                .handle_event(&event, &mut editor)
                .unwrap_or_else(|| handle_event(&mut editor, event, visible_rows));
            match event_res {
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
