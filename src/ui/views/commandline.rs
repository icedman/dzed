use crate::ui::layout::Rect;
use crate::ui::views::View;
use crate::{controller::controllers::ViewController, editor::Editor};
use std::io::Write;

use collections::Equivalent;
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};

pub struct CommandLineView;

impl CommandLineView {
    pub fn new() -> Self {
        CommandLineView {}
    }
}

impl CommandLineView {
    fn draw_commandline<W: Write>(
        &self,
        w: &mut W,
        rect: Rect,
        editor: &Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
        _doc: Option<&crate::editor::document::Document>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let remaining = rect
            .width
            .saturating_sub(editor.last_action.to_string().len() as u16);
        let status = format!("{}{}", editor.mode as usize, " ".repeat(remaining as usize));
        execute!(
            w,
            MoveTo(rect.x, rect.y),
            SetForegroundColor(Color::White),
            SetBackgroundColor(Color::Black),
            Print(status),
            ResetColor,
        )?;
        Ok(())
    }
}

impl View for CommandLineView {
    fn draw(
        &self,
        mut w: &mut dyn Write,
        rect: Rect,
        editor: &Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
        _doc: Option<&crate::editor::document::Document>,
        _ui: &crate::ui::Ui,
    ) -> Result<Option<(u16, u16, Option<crate::ui::CursorShape>)>, Box<dyn std::error::Error>> {
        self.draw_commandline(&mut w, rect, editor, buffer_manager, _doc)?;
        Ok(None)
    }
}
