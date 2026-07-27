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

    let tabline_bg = editor
        .colorscheme
        .ui
        .get("tabline_background")
        .map(|s| s.color)
        .unwrap_or(editor.theme.bg);
    let tabline_fg = editor
        .colorscheme
        .ui
        .get("tabline_foreground")
        .map(|s| s.color)
        .unwrap_or(editor.theme.fg);

    let tabline_sel_bg = editor
        .colorscheme
        .ui
        .get("tabline_sel_background")
        .map(|s| s.color)
        .unwrap_or(editor.theme.bg);
    let tabline_sel_fg = editor
        .colorscheme
        .ui
        .get("tabline_sel_foreground")
        .map(|s| s.color)
        .unwrap_or(editor.theme.fg);

    let tabline_fill_bg = editor
        .colorscheme
        .ui
        .get("tabline_fill")
        .map(|s| s.color)
        .unwrap_or(tabline_bg);

    execute!(
        w,
        MoveTo(rect.x, rect.y),
        SetBackgroundColor(tabline_bg),
        SetForegroundColor(tabline_fg)
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
                SetBackgroundColor(tabline_sel_bg),
                SetForegroundColor(tabline_sel_fg)
            )?;
        } else {
            execute!(
                w,
                SetBackgroundColor(tabline_bg),
                SetForegroundColor(tabline_fg)
            )?;
        }

        execute!(w, Print(&tab_text))?;
        cols_drawn += tab_text.chars().count();
    }

    // Fill remaining columns in the tab bar
    if (rect.width as usize) > cols_drawn {
        execute!(
            w,
            SetBackgroundColor(tabline_fill_bg),
            SetForegroundColor(tabline_fg)
        )?;
        execute!(w, Print(" ".repeat(rect.width as usize - cols_drawn)))?;
    }

    execute!(w, ResetColor)?;
    Ok(())
}
