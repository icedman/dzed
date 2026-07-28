use super::layout::Rect;
use super::view::View;
use crate::actions::Mode;
use crate::buffers::TextBuffer;
use crate::display::display_map::DisplayPoint;
use crate::document::BufferText;
use crate::editor::{AppContext, Editor};
use crate::search::{TextSearch, compile};
use crate::theme::{ColorAdjust, ToCrossTerm};
use crossterm::{cursor::MoveTo, execute};
use std::io::Write;
use text::ToPoint;

/// A standard view that renders the active text editor buffer.
pub struct TextView {
    pub window_id: usize,
}

impl TextView {
    pub fn new(window_id: usize) -> Self {
        TextView { window_id }
    }
}

impl View for TextView {
    fn draw(
        &mut self,
        mut w: &mut dyn Write,
        rect: Rect,
        editor: &mut Editor,
    ) -> std::io::Result<()> {
        let active_buffer = editor.buffer_manager.active_mut();
        let row_count = active_buffer.doc.buffer().row_count();
        let gutter_width = if editor.settings.show_line_numbers {
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
        Some(crate::input::handle_event(editor, event.clone()))
    }

    fn update(
        &mut self,
        ctx: &mut AppContext,
        editor: &mut Editor,
        rect: Rect,
        should_sync: &mut bool,
    ) -> std::io::Result<()> {
        let active_buffer = editor.buffer_manager.active_mut();

        // Update layout before wrapping so the wrap width reflects the current gutter.
        let row_count = active_buffer.doc.buffer().row_count();
        let gutter_width = if ctx.show_line_numbers {
            2 + if row_count == 0 {
                0
            } else {
                row_count.ilog10() as usize
            }
        } else {
            0
        };

        active_buffer.display_map.margin_left = gutter_width as u32;
        let wrap_cols = (rect.width as i32)
            .saturating_sub(active_buffer.display_map.margin_left as i32)
            .saturating_sub(active_buffer.display_map.margin_right as i32)
            .max(1);
        active_buffer
            .display_map
            .set_wrap_width(ctx.wrap.then_some(wrap_cols as u32));

        if *should_sync {
            active_buffer.display_map.fold(
                active_buffer.doc.folds.clone(),
                active_buffer.doc.buffer().snapshot().clone(),
            );

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
                .spawn_task(crate::background::BackgroundTask::Highlight {
                    owner_id: self.window_id,
                    file_path: active_buffer.file_path.clone(),
                    snapshot: active_buffer.doc.buffer().snapshot().clone(),
                    start_row: start,
                    row_count: active_buffer.doc.buffer().row_count() - start,
                    colorscheme: std::sync::Arc::new(editor.colorscheme.clone()),
                    theme: std::sync::Arc::new(editor.theme.theme.clone()),
                    use_colorscheme: ctx.use_colorscheme,
                    task_id: crate::background::TaskId(hl_task_id),
                    latest_task_id: active_buffer.latest_hl_task_id.clone(),
                });

            // Spawn background wrap task
            let wrap_width = ctx.wrap.then_some(wrap_cols as u32);
            let wrap_task_id = active_buffer
                .latest_wrap_task_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            editor
                .bg_worker
                .spawn_task(crate::background::BackgroundTask::Wrap {
                    owner_id: self.window_id,
                    file_path: active_buffer.file_path.clone(),
                    snapshot: active_buffer.doc.buffer().snapshot().clone(),
                    folds: active_buffer.doc.folds.clone(),
                    wrap_width,
                    task_id: crate::background::TaskId(wrap_task_id),
                    latest_task_id: active_buffer.latest_wrap_task_id.clone(),
                });

            if ctx.tree_sitter
                && let Some(grammar) = active_buffer.grammar
            {
                let parse_task_id = active_buffer
                    .latest_parse_task_id
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                    + 1;
                editor
                    .bg_worker
                    .spawn_task(crate::background::BackgroundTask::Parse {
                        owner_id: self.window_id,
                        file_path: active_buffer.file_path.clone(),
                        snapshot: active_buffer.doc.buffer().snapshot().clone(),
                        grammar,
                        task_id: crate::background::TaskId(parse_task_id),
                        latest_task_id: active_buffer.latest_parse_task_id.clone(),
                    });
            }

            *should_sync = false;
        }

        let cursor = active_buffer.doc.selection();
        let cursor_point = cursor.head().to_point(active_buffer.doc.buffer());
        let display_cursor = active_buffer
            .display_map
            .snapshot()
            .point_to_display_point(cursor_point);
        active_buffer.display_map.scroll_to_cursor(
            display_cursor,
            rect.height as i32,
            rect.width as i32,
        );

        // highlighting code
        let active_buffer = editor.buffer_manager.active_mut();
        let display_snapshot = active_buffer.display_map.snapshot();
        let total_rows = display_snapshot.row_count();
        let end_line =
            (display_snapshot.scroll_y + display_snapshot.visible_rows + 4).min(total_rows);

        if ctx.syntax && end_line > display_snapshot.scroll_y {
            let start_buffer_row =
                display_snapshot.buffer_row_for_display_row(display_snapshot.scroll_y);
            let last_visible_display_row = end_line.saturating_sub(1);
            let end_point = display_snapshot.display_point_to_point(DisplayPoint::new(
                last_visible_display_row,
                display_snapshot.line_len(last_visible_display_row),
            ));
            let end_buffer_row = end_point.row;
            let end_buffer_row_exclusive = end_buffer_row + 1;

            if !active_buffer
                .hl
                .is_sync(&active_buffer.doc.buffer().snapshot())
                || !active_buffer
                    .hl
                    .contains_rows(start_buffer_row, end_buffer_row_exclusive)
            {
                active_buffer.hl.highlight_lines(
                    &active_buffer.doc.buffer().snapshot(),
                    start_buffer_row,
                    end_buffer_row_exclusive - start_buffer_row,
                    &editor.colorscheme,
                    &editor.theme.theme,
                    ctx.use_colorscheme,
                );
            }
        }

        Ok(())
    }

    fn handle_task(
        &mut self,
        result: &crate::background::BackgroundResult,
        editor: &mut Editor,
    ) -> std::io::Result<()> {
        match result {
            crate::background::BackgroundResult::HighlightComplete {
                file_path,
                style_cache,
                task_id,
                ..
            } => {
                if let Some(buf) = editor
                    .buffer_manager
                    .buffers
                    .iter_mut()
                    .find(|b| &b.file_path == file_path)
                {
                    if *task_id >= crate::background::TaskId(buf.current_hl_task_id) {
                        buf.current_hl_task_id = task_id.0;
                        buf.hl
                            .merge_caches(style_cache.clone(), std::collections::HashMap::new());
                    }
                }
            }
            crate::background::BackgroundResult::WrapComplete {
                file_path,
                wrap_snapshot,
                task_id,
                ..
            } => {
                if let Some(buf) = editor
                    .buffer_manager
                    .buffers
                    .iter_mut()
                    .find(|b| &b.file_path == file_path)
                {
                    if *task_id >= crate::background::TaskId(buf.current_wrap_task_id) {
                        buf.current_wrap_task_id = task_id.0;
                        buf.display_map.apply_wrap_snapshot(wrap_snapshot.clone());
                    }
                }
            }
            crate::background::BackgroundResult::ParseComplete {
                file_path,
                syntax_tree,
                task_id,
                ..
            } => {
                if editor.settings.tree_sitter {
                    if let Some(buf) = editor
                        .buffer_manager
                        .buffers
                        .iter_mut()
                        .find(|b| &b.file_path == file_path)
                    {
                        if *task_id >= crate::background::TaskId(buf.current_parse_task_id) {
                            buf.current_parse_task_id = task_id.0;
                            buf.syntax_tree = Some(syntax_tree.clone());
                        }
                    }
                }
            }
        }
        Ok(())
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

    use crate::theme::ToCrossTerm;
    let theme_fg = editor
        .theme
        .theme
        .settings
        .foreground
        .map(|c| c.rgb())
        .unwrap_or(crossterm::style::Color::White);
    let theme_bg = editor
        .theme
        .theme
        .settings
        .background
        .map(|c| c.rgb())
        .unwrap_or(crossterm::style::Color::Black);
    let theme_caret = editor
        .theme
        .theme
        .settings
        .caret
        .map(|c| c.rgb())
        .unwrap_or(theme_fg);
    let theme_select = editor
        .theme
        .theme
        .settings
        .selection
        .map(|c| c.rgb())
        .unwrap_or(theme_bg);

    let (editor_fg, editor_bg, caret_bg, caret_fg, selection_bg) =
        if editor.settings.use_colorscheme {
            let fg = editor
                .colorscheme
                .ui
                .get("foreground")
                .map(|s| s.color)
                .unwrap_or(theme_fg);
            let bg = editor
                .colorscheme
                .ui
                .get("background")
                .map(|s| s.color)
                .unwrap_or(theme_bg);
            let sel = editor
                .colorscheme
                .ui
                .get("selection")
                .map(|s| s.color)
                .unwrap_or(theme_select);
            let c_bg = editor
                .colorscheme
                .ui
                .get("caret")
                .map(|s| s.color)
                .unwrap_or(sel);
            let c_fg = fg; // editor.colorscheme.ui.get("caret_foreground").map(|s| s.color).unwrap_or(bg);
            (fg, bg, c_bg, c_fg, sel)
        } else {
            (theme_fg, theme_bg, theme_caret, theme_bg, theme_select)
        };

    let gutter_fg = editor
        .colorscheme
        .ui
        .get("gutter_foreground")
        .map(|s| s.color)
        .unwrap_or(editor_fg);
    let gutter_bg = editor
        .colorscheme
        .ui
        .get("gutter")
        .map(|s| s.color)
        .unwrap_or(editor_bg);
    let find_fg = editor
        .colorscheme
        .ui
        .get("find_highlight_foreground")
        .map(|s| s.color)
        .unwrap_or(editor_fg);
    let find_bg = editor
        .colorscheme
        .ui
        .get("find_highlight")
        .map(|s| s.color)
        .unwrap_or(selection_bg);

    let mut prev_line_number = -1;
    let mut screen_row = inner_rect.y;

    // Scrollbar metrics
    let track_bg = gutter_bg;
    let handle_bg = selection_bg;

    let height = inner_rect.height as u32;
    let handle_h = if total_rows > 0 {
        ((height as f32 / total_rows as f32) * height as f32)
            .round()
            .max(1.0) as u32
    } else {
        height
    };
    let handle_h = handle_h.min(height);

    let start_y = if total_rows > height {
        ((display_snapshot.scroll_y as f32 / (total_rows - height) as f32)
            * (height - handle_h) as f32)
            .round() as u32
    } else {
        0
    };

    for row in display_snapshot.scroll_y..end_line {
        execute!(stdout, MoveTo(inner_rect.x, screen_row)).unwrap();

        // line number
        if editor.settings.show_line_numbers {
            let line_number = display_snapshot.buffer_row_for_display_row(row);
            execute!(stdout, crossterm::style::SetForegroundColor(gutter_fg)).unwrap();

            execute!(stdout, crossterm::style::SetBackgroundColor(gutter_bg)).unwrap();
            if prev_line_number != line_number as i32 {
                print!("{:>width$} ", (line_number + 1), width = gutter_width - 1);
            } else {
                print!("{}", " ".repeat(gutter_width));
            }
            prev_line_number = line_number as i32;
        }

        let text = display_snapshot.line_text(row) + " ";

        let mut matches = Vec::<(usize, usize, &str)>::new();
        if editor.command.pattern {
            if editor.command.search_text != editor.command.regex_string {
                editor.command.regex_string = editor.command.search_text.clone();
                editor.command.regex = compile(editor.command.regex_string.as_str());
            }
            if let Some(ref regex) = editor.command.regex {
                matches = text.as_str().find_pattern(&regex);
            }
        } else if !editor.command.search_text.is_empty() {
            matches = text.as_str().find_string(&editor.command.search_text);
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

        let mut x_scroll = display_snapshot.scroll_x;
        let mut cols_remaining = (inner_rect.width as usize).saturating_sub(gutter_width);

        let mut curr_x = inner_rect.x + gutter_width as u16;
        let relative_row = (screen_row - inner_rect.y) as u32;
        let is_handle = relative_row >= start_y && relative_row < start_y + handle_h;

        for (column, ch) in text.chars().enumerate() {
            let orig_point =
                display_snapshot.display_point_to_point(DisplayPoint::new(row, column as u32));
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

            let mut fg = editor_fg;
            let mut bg = editor_bg;

            if editor.settings.syntax {
                if let Some(style_cache) = active_buffer.hl.render_row(orig_point.row) {
                    if let Some(&(style, _, _)) =
                        style_cache.styles.iter().find(|(_, start, end)| {
                            orig_point.column >= *start && orig_point.column < *end
                        })
                    {
                        fg = style.foreground.rgb();
                        bg = style.background.rgb();
                    }
                }
            }

            // Apply search match background if not in a selection
            if in_match {
                fg = find_fg;
                bg = find_bg;
            }

            let (selected, mut selected_line, at_cursor) = active_buffer
                .doc
                .selections()
                .is_selected(orig_point.row, orig_point.column, &buffer);
            if selected && (editor.mode != Mode::Command) {
                bg = selection_bg;
            }
            selected_line = selected_line && editor.mode == Mode::VisualLine;
            if selected_line {
                bg = selection_bg;
            }

            if at_cursor {
                bg = selection_bg;
                // bg = caret_bg;
                // fg = caret_fg;
            }

            if x_scroll > 0 {
                x_scroll = x_scroll.saturating_sub(1);
            } else {
                let is_scrollbar = curr_x == inner_rect.x + inner_rect.width - 1;
                let bg_color = if is_scrollbar {
                    if is_handle { handle_bg } else { track_bg }
                } else {
                    bg
                };

                execute!(stdout, crossterm::style::SetForegroundColor(fg)).unwrap();
                execute!(stdout, crossterm::style::SetBackgroundColor(bg_color)).unwrap();

                match ch {
                    '\t' => {
                        for _i in 0..4 {
                            // Tab size of 4
                            let is_scrollbar_tab = curr_x == inner_rect.x + inner_rect.width - 1;
                            let cell_bg = if is_scrollbar_tab {
                                if is_handle { handle_bg } else { track_bg }
                            } else if at_cursor
                                && editor.mode != Mode::Insert
                                && editor.mode != Mode::Command
                            {
                                editor_bg
                            } else {
                                bg
                            };
                            execute!(stdout, crossterm::style::SetBackgroundColor(cell_bg))
                                .unwrap();
                            print!(" ");
                            curr_x += 1;
                            cols_remaining = cols_remaining.saturating_sub(1);
                        }
                    }
                    _ => {
                        print!("{}", ch);
                        curr_x += 1;
                        cols_remaining = cols_remaining.saturating_sub(1);
                    }
                }
            }

            if cols_remaining <= 0 {
                break;
            }
        }

        for _ in 0..cols_remaining {
            let is_scrollbar = curr_x == inner_rect.x + inner_rect.width - 1;
            let bg_color = if is_scrollbar {
                if is_handle { handle_bg } else { track_bg }
            } else {
                editor_bg
            };
            execute!(stdout, crossterm::style::SetBackgroundColor(bg_color)).unwrap();
            print!(" ");
            curr_x += 1;
        }

        screen_row += 1;
        if screen_row >= inner_rect.y + inner_rect.height {
            break;
        }
    }

    // Clear and draw scrollbar for any remaining empty rows in the viewport
    while screen_row < inner_rect.y + inner_rect.height {
        execute!(stdout, MoveTo(inner_rect.x, screen_row)).unwrap();
        if editor.settings.show_line_numbers {
            execute!(
                stdout,
                crossterm::style::SetForegroundColor(gutter_fg),
                crossterm::style::SetBackgroundColor(gutter_bg)
            )
            .unwrap();
            print!("~");
            print!("{}", " ".repeat(gutter_width - 1));
        }

        let mut curr_x = inner_rect.x + gutter_width as u16;
        let cols_remaining = (inner_rect.width as usize).saturating_sub(gutter_width);
        let relative_row = (screen_row - inner_rect.y) as u32;
        let is_handle = relative_row >= start_y && relative_row < start_y + handle_h;

        for _ in 0..cols_remaining {
            let is_scrollbar = curr_x == inner_rect.x + inner_rect.width - 1;
            let bg_color = if is_scrollbar {
                if is_handle { handle_bg } else { track_bg }
            } else {
                editor_bg
            };
            execute!(stdout, crossterm::style::SetBackgroundColor(bg_color)).unwrap();
            print!(" ");
            curr_x += 1;
        }
        screen_row += 1;
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
    show_cursor: bool,
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

    let (ox, oy) = (0, 0);

    if editor.mode == Mode::Command {
        let cmd_text = editor.command.get_text();
        let cmd_col = (cmd_text.chars().count()) as u16;
        // Command line is drawn at statusbar rect y position (bottom row)
        execute!(
            stdout,
            MoveTo(cmd_col + 1, (editor.settings.screen_rows - 1) as u16),
        )
        .unwrap();
    } else {
        execute!(
            stdout,
            MoveTo(
                inner_rect.x - ox + gutter_width as u16 + cursor_screen_col as u16,
                inner_rect.y - oy + cursor_screen_row as u16
            ),
        )
        .unwrap();
    }

    if show_cursor {
        execute!(stdout, crossterm::cursor::Show).unwrap();
    } else {
        execute!(stdout, crossterm::cursor::Hide).unwrap();
    }
    Ok(())
}
