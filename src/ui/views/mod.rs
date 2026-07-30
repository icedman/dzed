pub mod commandline;
pub mod statusbar;
pub mod tabs;
pub mod textview;

use crate::editor::Editor;
use crate::ui::layout::Rect;
use std::io::Write;

pub trait View {
    fn draw(
        &self,
        w: &mut dyn Write,
        rect: Rect,
        editor: &Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
        doc: Option<&crate::editor::document::Document>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
