use crate::controller::ControllerResult;
use crate::controller::actions;
use crate::editor::Editor;
use crate::ui::Ui;

pub mod tabscontroller;
pub mod textview;

pub trait ViewController {
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
