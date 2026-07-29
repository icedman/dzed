use crate::editor::Editor;
use crate::ui::layout::Rect;
use std::io::Write;

pub trait View {
    fn draw(
        &mut self,
        w: &mut dyn Write,
        rect: Rect,
        editor: &Editor,
    ) -> Result<(), Box<dyn std::error::Error>>;
}
