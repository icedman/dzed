use super::layout::Rect;
use crate::editor::{AppContext, Editor};
use crossterm::event::Event;
use std::io::Write;

pub trait View {
    fn draw(&mut self, w: &mut dyn Write, rect: Rect, editor: &mut Editor) -> std::io::Result<()>;
    fn handle_event(
        &mut self,
        event: &Event,
        editor: &mut Editor,
    ) -> Option<crate::input::HandleEvent> {
        None
    }
    fn update(
        &mut self,
        _ctx: &mut AppContext,
        _editor: &mut Editor,
        _rect: Rect,
        _should_sync: &mut bool,
    ) -> std::io::Result<()> {
        Ok(())
    }
    fn handle_task(
        &mut self,
        _result: &crate::background::BackgroundResult,
        _editor: &mut Editor,
    ) -> std::io::Result<()> {
        Ok(())
    }
}
