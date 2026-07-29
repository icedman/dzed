use crate::editor::Editor;
use crate::ui::layout::Rect;
use crate::ui::views::View;

use std::io::Write;

use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};

pub struct TextView {}

impl TextView {
    pub fn new() -> Self {
        TextView {}
    }
}

impl TextView {
    fn draw_tabs<W: Write>(
        &self,
        w: &mut W,
        rect: Rect,
        editor: &Editor,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let buffer = editor.buffer_manager.active();
        let rows = buffer.doc.buffer().row_count();

        execute!(
            w,
            MoveTo(rect.x, rect.y),
            SetForegroundColor(Color::Black),
            SetBackgroundColor(Color::White),
            Print(format!("Buffer...{} rows", rows)),
            ResetColor,
        )?;

        Ok(())
    }
}

impl View for TextView {
    fn draw(
        &self,
        mut w: &mut dyn Write,
        rect: Rect,
        editor: &Editor,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.draw_tabs(&mut w, rect, editor)
    }
}
