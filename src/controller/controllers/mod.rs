use crate::controller::ControllerResult;
use crate::controller::actions;
use crate::editor::Editor;
use crate::ui::Ui;
use crate::ui::layout::Rect;

use crate::services::background;

pub mod commandline;
pub mod tabs;
pub mod textview;

pub trait ViewController {
    fn update(
        &self,
        editor: &mut Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
        ui: &Ui,
        window_id: usize,
        rect: Rect,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        Ok(ControllerResult::None)
    }

    fn handle_action(
        &self,
        action: actions::Action,
        editor: &mut Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
        ui: &Ui,
        window_id: usize,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        Ok(ControllerResult::None)
    }

    fn handle_task(
        &mut self,
        _result: &background::BackgroundResult,
        _editor: &mut Editor,
        _buffer_manager: &mut crate::editor::buffers::BufferManager,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        Ok(ControllerResult::None)
    }
}
