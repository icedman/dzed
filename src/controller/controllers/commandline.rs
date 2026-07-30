use crate::controller::ControllerResult;
use crate::controller::ViewController;
use crate::controller::actions::Action;
use crate::editor::Editor;
use crate::ui::Ui;

pub struct CommandLineController {}

impl CommandLineController {
    pub fn new() -> Self {
        CommandLineController {}
    }
}

impl ViewController for CommandLineController {
    fn handle_action(
        &self,
        action: Action,
        editor: &mut Editor,
        ui: &Ui,
        window_id: usize,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        Ok(ControllerResult::None)
    }
}

