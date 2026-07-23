use crate::editor::Editor;
use super::layout::Rect;
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};
use std::io::Write;

pub struct Tabs;

impl Tabs {
    pub fn new() -> Self {
        Tabs
    }

    pub fn draw<W: Write>(&self, w: &mut W, rect: Rect, editor: &Editor) -> std::io::Result<()> {
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
}
