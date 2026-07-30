use crate::controller::ControllerResult;
use crate::controller::actions;
use crate::editor::Editor;
use crate::ui::Ui;
use crate::ui::layout::Rect;

pub mod tabs;
pub mod textview;

pub trait ViewController {
    fn update(
        &self,
        editor: &mut Editor,
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
        ui: &Ui,
        window_id: usize,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        Ok(ControllerResult::None)
    }
}
