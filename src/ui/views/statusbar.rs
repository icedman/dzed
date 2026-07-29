use crate::editor::Editor;
use crate::ui::layout::Rect;
use crate::ui::views::View;
use std::io::Write;

use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};

pub struct StatusBarView;

impl StatusBarView {
    pub fn new() -> Self {
        StatusBarView
    }
}

impl StatusBarView {
    fn draw_statusbar<W: Write>(
        &mut self,
        w: &mut W,
        rect: Rect,
        editor: &Editor,
    ) -> Result<(), Box<dyn std::error::Error>> {
        execute!(
            w,
            MoveTo(rect.x, rect.y),
            SetForegroundColor(Color::Black),
            SetBackgroundColor(Color::White),
            Print("STATUS"),
            ResetColor,
        )?;
        Ok(())
    }
}

impl View for StatusBarView {
    fn draw(
        &mut self,
        mut w: &mut dyn Write,
        rect: Rect,
        editor: &Editor,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.draw_statusbar(&mut w, rect, editor)
    }
}
