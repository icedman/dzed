use crate::editor::{Editor, document::BufferText};
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
    fn draw_textview<W: Write>(
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
            Print(format!("Buffer...{} rows\r\n", rows)),
            ResetColor,
        )?;

        for row in 0..rows {
            let text = buffer.doc.buffer().row_text(row);
            execute!(
                w,
                MoveTo(rect.x, rect.y + row as u16 + 1),
                Print(format!("{}\r\n", text)),
                ResetColor,
            )?;
            if row > 8 {
                break;
            }
        }

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
        self.draw_textview(&mut w, rect, editor)
    }
}
