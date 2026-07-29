use crate::controller::ControllerResult;
use crate::controller::ViewController;

use crate::editor::Editor;
use crate::ui::layout::Rect;
use std::io::Write;
pub struct TabsController {}

impl TabsController {
    pub fn new() -> Self {
        TabsController {}
    }
}

impl ViewController for TabsController {
    fn handle_action(
        &self,
        action: crate::controller::actions::Action,
        editor: &mut Editor,
        ui: &crate::ui::Ui,
        window_id: usize,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        Ok(ControllerResult::None)
    }
}
