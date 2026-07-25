use super::layout::Rect;
use crate::editor::Editor;
use crossterm::event::Event;
use std::io::Write;

pub trait View {
    fn draw(&mut self, w: &mut dyn Write, rect: Rect, editor: &mut Editor) -> std::io::Result<()>;
    fn handle_event(
        &mut self,
        event: &Event,
        editor: &mut Editor,
    ) -> Option<crate::input::HandleEvent>;
    fn update(
        &mut self,
        _editor: &mut Editor,
        _rect: Rect,
        _should_sync: &mut bool,
    ) -> std::io::Result<()> {
        Ok(())
    }
}
