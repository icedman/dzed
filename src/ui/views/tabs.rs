use crate::ui::layout::Rect;
use crate::ui::views::View;
use crate::{controller::controllers::ViewController, editor::Editor};
use std::io::Write;

use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};

pub struct TabsView;

impl TabsView {
    pub fn new() -> Self {
        TabsView {}
    }
}

impl TabsView {
    fn draw_tabs<W: Write>(
        &self,
        w: &mut W,
        rect: Rect,
        editor: &Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
    ) -> Result<(), Box<dyn std::error::Error>> {
        execute!(
            w,
            MoveTo(rect.x, rect.y),
            SetForegroundColor(Color::Black),
            SetBackgroundColor(Color::White),
            Print("TABS"),
            ResetColor,
        )?;

        for (idx, buf) in buffer_manager.buffers.iter().enumerate() {
            let name = if buf.file_path.is_empty() {
                "[No Name]".to_string()
            } else {
                std::path::Path::new(&buf.file_path)
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new(&buf.file_path))
                    .to_string_lossy()
                    .into_owned()
            };

            let is_active = idx == buffer_manager.active_idx;
            let tab_text = if is_active {
                format!(" [{}] ", name)
            } else {
                format!("  {}  ", name)
            };

            execute!(w, Print(tab_text), ResetColor,)?;
        }

        Ok(())
    }
}

impl View for TabsView {
    fn draw(
        &self,
        mut w: &mut dyn Write,
        rect: Rect,
        editor: &Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.draw_tabs(&mut w, rect, editor, buffer_manager)
    }
}
