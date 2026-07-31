use crate::controller::ControllerResult;
use crate::controller::ViewController;
use crate::controller::actions::Action;
use crate::editor::Editor;
use crate::services::background;
use crate::ui::Ui;
use crate::ui::layout::Rect;
use std::io::Write;

pub struct CommandLineController {
    controller: crate::controller::controllers::textview::TextViewController,
}

impl CommandLineController {
    pub fn new() -> Self {
        CommandLineController {
            controller: crate::controller::controllers::textview::TextViewController::new(),
        }
    }
}

impl ViewController for CommandLineController {
    fn update(
        &self,
        editor: &mut Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
        ui: &mut crate::ui::Ui,
        window_id: usize,
        rect: Rect,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        self.controller
            .update(editor, buffer_manager, ui, window_id, rect)
    }

    fn handle_action(
        &self,
        action: Action,
        editor: &mut Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
        ui: &mut crate::ui::Ui,
        window_id: usize,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        match action {
            Action::InsertNewLine { .. } => {
                let mut command_text = String::new();
                if let Some(window) = ui.windows.get_mut(&window_id) {
                    if let Some(ref mut document) = window.doc {
                        if let Some(buffer) = buffer_manager.find_mut(document) {
                            command_text = buffer.buffer.snapshot().text();
                            buffer.clear();
                        }
                        if let Some(buffer) = buffer_manager.find(document) {
                            document.clear(&buffer.buffer);
                        }
                    }
                }
                let mut command_text = command_text
                    .trim_end_matches(|c| c == '\r' || c == '\n')
                    .to_string();
                if command_text.starts_with(':') {
                    command_text = command_text[1..].to_string();
                }
                return Ok(ControllerResult::Command(command_text));
            }
            _ => {}
        }
        self.controller
            .handle_action(action, editor, buffer_manager, ui, window_id)
    }

    fn handle_task(
        &mut self,
        result: &background::BackgroundResult,
        editor: &mut Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
        doc: Option<&mut crate::editor::document::Document>,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        self.controller
            .handle_task(result, editor, buffer_manager, doc)
    }
}
