mod actions;
mod display;
mod document;
mod editor;
mod highlight;
mod input;
mod selections;

use std::{
    io::{Write, stdout},
    time::Duration,
};

use crossterm::{
    cursor::MoveTo,
    event::{self, Event},
    execute,
    terminal::{Clear, ClearType},
};

use text::ToPoint;

use actions::{Action, Mode};
use document::{BufferText, Document};
use editor::{ColorAdjust, Editor, EditorBuffer, EditorTheme, ToCrossTerm};
use input::{HandleEvent, handle_event};

fn fill_to_eol(count: usize) {
    for _ in 0..count {
        print!(" ");
    }
}

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

    let tab_size = 4;
    execute!(stdout, crossterm::cursor::Hide).unwrap();

    let mut should_redraw = true;
    let mut prev_screen_rows = 0;
    let mut prev_screen_cols = 0;

    loop {
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

        let active_buffer = editor.buffer_manager.active_mut();

        // update display map
        let display_snapshot = active_buffer.display_map.snapshot();
        let wrap_cols = screen_cols
            .saturating_sub(display_snapshot.margin_left as i32)
            .saturating_sub(display_snapshot.margin_right as i32);

        active_buffer.display_map.set_wrap_width(if editor.wrap {
            Some(wrap_cols as u32)
        } else {
            None
        });
        active_buffer
            .display_map
            .sync(active_buffer.doc.buffer().snapshot().clone());

        // get cursor information
        let cursor = active_buffer.doc.selection();
        let cursor_head = cursor.head();
        let cursor_point = cursor_head.to_point(&active_buffer.doc.buffer());
        let display_cursor = active_buffer
            .display_map
            .snapshot()
            .point_to_display_point(cursor_point);

        active_buffer
            .display_map
            .scroll_to_cursor(display_cursor, screen_rows, screen_cols);

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

            if active_buffer.dirty_hl {
                let start_buffer_row =
                    display_snapshot.buffer_row_for_display_row(display_snapshot.scroll_y);
                let end_buffer_row =
                    display_snapshot.buffer_row_for_display_row(end_line.saturating_sub(1));

                active_buffer.hl.highlight_lines(
                    active_buffer.doc.buffer(),
                    start_buffer_row as usize,
                    (end_buffer_row - start_buffer_row + 1) as usize,
                );
            }
            active_buffer.dirty_hl = true;

            let mut prev_line_number = -1;
            let mut screen_row = display_snapshot.margin_top as u16;

            let default_style = active_buffer.hl.get_default_style();

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
                        crossterm::style::SetBackgroundColor(editor.theme.gutter_bg) // crossterm::style::SetBackgroundColor(
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
                let buffer_row = display_snapshot.buffer_row_for_display_row(row);
                let buffer_range = display_snapshot.buffer_range_for_display_row(row);
                let start_col = buffer_range.start.column;

                let ranges;
                if let Some(style_cache) = active_buffer.hl.render_line(buffer_row as usize) {
                    ranges = &style_cache.styles;
                } else {
                    execute!(
                        stdout,
                        crossterm::style::SetBackgroundColor(editor.theme.bg)
                    )
                    .unwrap();
                    fill_to_eol(screen_cols as usize);
                    screen_row += 1;
                    continue;
                }

                // style range
                let mut range_iter = ranges.iter();
                let mut current_range = range_iter.next();

                // Skip ranges that end before our start_col
                while let Some((_, _s, e)) = current_range {
                    if *e <= start_col {
                        current_range = range_iter.next();
                    } else {
                        break;
                    }
                }

                let mut range_remaining =
                    current_range.map_or(
                        0,
                        |(_, s, e)| {
                            if *s < start_col { e - start_col } else { e - s }
                        },
                    );

                let mut current_style = current_range.map(|(style, _, _)| style);
                if !editor.syntax {
                    current_style = Some(&default_style);
                }

                let mut x_scroll = display_snapshot.scroll_x;
                let mut cols_remaining = screen_cols
                    .saturating_sub(display_snapshot.margin_left as i32)
                    .saturating_sub(display_snapshot.margin_right as i32);

                for (column, ch) in text.chars().enumerate() {
                    let rc = start_col + column as u32;

                    if editor.syntax && range_remaining == 0 {
                        current_range = range_iter.next();
                        range_remaining = current_range.map_or(0, |(_, s, e)| e - s);
                        current_style = current_range.map(|(style, _, _)| style);
                    }

                    let mut fg = editor.theme.fg.clone();
                    let mut bg = editor.theme.bg.clone();

                    if let Some(style) = current_style {
                        fg = style.foreground.rgb();
                        bg = style.background.darken(10).rgb();
                    }

                    let (selected, mut selected_line, at_cursor) = active_buffer
                        .doc
                        .selections()
                        .is_selected(buffer_row, rc, &buffer);
                    if selected && (editor.mode == Mode::Visual || editor.mode == Mode::VisualLine)
                    {
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

                    range_remaining = range_remaining.saturating_sub(1);

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
                    crossterm::style::SetBackgroundColor(editor.theme.gutter_bg)
                )
                .unwrap();
                execute!(stdout, MoveTo(0, screen_rows as u16)).unwrap();
                fill_to_eol(screen_cols as usize);
                execute!(stdout, MoveTo(0, screen_rows as u16)).unwrap();

                if editor.mode == Mode::Command {
                    let mut cmd_char = ':';
                    if editor.search {
                        cmd_char = '/';
                        if editor.regex {
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
                        "[{}/{}] {} {} {},{} rl:{} {} {}",
                        active_idx + 1,
                        buffer_count,
                        active_buffer.file_path,
                        match editor.mode {
                            Mode::Normal => "NORMAL",
                            Mode::Insert => "INSERT",
                            Mode::Visual => "VISUAL",
                            Mode::VisualLine => "V-LINE",
                            Mode::Command => "COMMAND",
                        },
                        active_buffer.doc.selection().head().offset,
                        active_buffer.doc.selection().tail().offset,
                        row_len,
                        active_buffer.hl.name(),
                        editor.pending_cmd,
                    );
                }
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
                    crossterm::cursor::SetCursorStyle::BlinkingBar,
                    crossterm::cursor::Show
                )
                .unwrap();
            } else {
                execute!(
                    stdout,
                    MoveTo(
                        display_snapshot.margin_left as u16 + cursor_screen_col as u16,
                        display_snapshot.margin_top as u16 + cursor_screen_row as u16
                    ),
                    match editor.mode {
                        Mode::Insert => crossterm::cursor::SetCursorStyle::BlinkingBar,
                        _ => crossterm::cursor::SetCursorStyle::BlinkingBlock,
                    },
                    crossterm::cursor::Show
                )
                .unwrap();
            }

            stdout.flush().unwrap();
        }

        //------------------
        // input
        //------------------
        if event::poll(Duration::from_millis(50))? {
            let event = event::read()?;
            match handle_event(&mut editor, event, visible_rows) {
                HandleEvent::Exit => break,
                HandleEvent::Redraw => should_redraw = true,
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
    execute!(stdout, crossterm::cursor::Show).unwrap();

    Ok(())
}
