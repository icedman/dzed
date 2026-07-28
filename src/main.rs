mod controller;
mod editor;
mod services;
mod ui;

use std::{
    collections::HashMap,
    io::{Write, stdout},
    time::{Duration, Instant},
};

use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyEvent, MouseEventKind},
    execute,
    terminal::{Clear, ClearType},
};

pub struct BufferManager {}
pub struct LayoutManager {}

pub struct Application {
    pub buffers: BufferManager,
    pub layout: LayoutManager,
}

pub struct Controller {}
pub struct Views {}

pub struct Renderer {}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let file_paths = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        Vec::new()
    };

    let mut ui = ui::Ui::new();
    let mut editor = editor::Editor::new(file_paths);
    // let mut controller = controller::Controller::new();

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

    loop {
        //------------------
        // layout
        //------------------
        // get screen dimensions
        let (screen_cols, screen_rows) = {
            let (cols, rows) = crossterm::terminal::size().unwrap();
            (cols as i32, rows as i32)
        };
        ui.layout(screen_cols as u32, screen_rows as u32);

        //------------------
        // update
        //------------------

        //------------------
        // render
        //------------------
        let computed = &ui.cached_layouts;
        for &(win_id, rect) in computed {
            if let Some(win) = ui.windows.get_mut(&win_id) {
                win.draw(&mut stdout, rect)?;
            }
        }

        //------------------
        // input
        //------------------
        if event::poll(Duration::from_millis(50))? {
            let event = event::read()?;
            match event {
                Event::Key(key_event) => {
                    break;
                }
                _ => {}
            }
        }

        //------------------
        // controller
        //------------------
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
