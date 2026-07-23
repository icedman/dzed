use super::layout::Rect;
use super::view::View;
use crate::editor::Editor;
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};
use std::io::Write;

pub struct TabsView;

impl TabsView {
    pub fn new() -> Self {
        TabsView
    }
}

impl View for TabsView {
    fn draw(
        &mut self,
        mut w: &mut dyn Write,
        rect: Rect,
        editor: &mut Editor,
        _last_cursor_style: &mut Option<crossterm::cursor::SetCursorStyle>,
    ) -> std::io::Result<()> {
        draw_tabs_impl(&mut w, rect, editor)
    }

    fn handle_event(
        &mut self,
        _event: &crossterm::event::Event,
        _editor: &mut Editor,
    ) -> Option<crate::input::HandleEvent> {
        None
    }
}

fn draw_tabs_impl<W: Write>(w: &mut W, rect: Rect, editor: &Editor) -> std::io::Result<()> {
    if rect.width == 0 || rect.height == 0 {
        return Ok(());
    }

    execute!(
        w,
        MoveTo(rect.x, rect.y),
        SetBackgroundColor(editor.theme.gutter),
        SetForegroundColor(editor.theme.gutter_fg)
    )?;

    let mut cols_drawn = 0usize;

    for (idx, buf) in editor.buffer_manager.buffers.iter().enumerate() {
        let name = if buf.file_path.is_empty() {
            "[No Name]".to_string()
        } else {
            std::path::Path::new(&buf.file_path)
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new(&buf.file_path))
                .to_string_lossy()
                .into_owned()
        };

        let is_active = idx == editor.buffer_manager.active_idx;
        let tab_text = if is_active {
            format!(" [{}] ", name)
        } else {
            format!("  {}  ", name)
        };

        if is_active {
            execute!(
                w,
                SetBackgroundColor(editor.theme.bg),
                SetForegroundColor(editor.theme.fg)
            )?;
        } else {
            execute!(
                w,
                SetBackgroundColor(editor.theme.gutter),
                SetForegroundColor(editor.theme.gutter_fg)
            )?;
        }

        execute!(w, Print(&tab_text))?;
        cols_drawn += tab_text.chars().count();
    }

    // Fill remaining columns in the tab bar
    if (rect.width as usize) > cols_drawn {
        execute!(
            w,
            SetBackgroundColor(editor.theme.gutter),
            SetForegroundColor(editor.theme.gutter_fg)
        )?;
        execute!(w, Print(" ".repeat(rect.width as usize - cols_drawn)))?;
    }

    execute!(w, ResetColor)?;
    Ok(())
}
