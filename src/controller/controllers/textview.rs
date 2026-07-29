use crate::controller::ControllerResult;
use crate::controller::ViewController;

use crate::editor::Editor;
use crate::ui::layout::Rect;
use std::io::Write;

pub struct TextViewController {}

impl TextViewController {
    pub fn new() -> Self {
        TextViewController {}
    }
}

impl ViewController for TextViewController {
    fn handle_action(
        &self,
        action: crate::controller::actions::Action,
        editor: &mut Editor,
        ui: &crate::ui::Ui,
        window_id: usize,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        let buffer = editor.buffer_manager.active_mut();
        editor.apply_active_action(&action);
        Ok(ControllerResult::None)
    }
}
