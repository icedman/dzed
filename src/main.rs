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

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let file_paths = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        Vec::new()
    };

    let mut ui = ui::Ui::new();
    let mut editor = editor::Editor::new(file_paths)?;
    let mut controller = controller::Controller::new();

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
        ui.update(&mut editor)?;
        if editor.should_redraw {
            ui.draw(&mut stdout, &mut editor)?;
        }

        if event::poll(Duration::from_millis(50))? {
            let event = event::read()?;
            match controller.handle_event(event, &mut editor)? {
                controller::ControllerResult::Exit => {
                    break;
                }
                _ => {}
            }
        }

        controller.dispatch_actions(&mut editor, &mut ui)?;

        //------------------
        // 4. Background work
        //------------------
        // editor.services.poll(&mut editor)?;
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
