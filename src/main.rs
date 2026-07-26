mod actions;
mod background;
mod clipboard;
mod command;
mod display;
mod document;
mod editor;
mod ex;
mod exmap;
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
    let mut last_activity = Instant::now();
    let mut cursor_visible = false;

    loop {
        let start = Instant::now();

        editor.mode = editor.buffer_manager.active().doc.current_mode();

        //------------------
        // Drain any incoming background worker results
        //------------------
        while let Some(result) = editor.bg_worker.try_recv() {
            let owner_id = match &result {
                background::BackgroundResult::HighlightComplete { owner_id, .. } => *owner_id,
                background::BackgroundResult::WrapComplete { owner_id, .. } => *owner_id,
                background::BackgroundResult::ParseComplete { owner_id, .. } => *owner_id,
            };
            if let Some(win) = ui.windows.get_mut(&owner_id) {
                if let Some(ref mut view) = win.view {
                    let _ = view.handle_task(&result, &mut editor);
                    should_redraw = true;
                }
            }
        }

        //------------------
        // layout
        //------------------
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

        //------------------
        // update
        //------------------
        ui.update(&mut editor, &mut should_sync)?;

        if !cursor_visible && last_activity.elapsed() >= Duration::from_millis(250) {
            cursor_visible = true;
            should_redraw = true;
        }

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
                cursor_visible,
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
            last_activity = Instant::now();
            cursor_visible = false;
            let event_res = ui
                .handle_event(&event, &mut editor)
                .unwrap_or_else(|| handle_event(&mut editor, event));
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
