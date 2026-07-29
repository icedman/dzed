pub mod buffers;
pub mod display;
pub mod document;
pub mod selections;

use crate::controller::{self};
use crate::editor::buffers::BufferManager;
use crate::editor::buffers::TextBuffer;
use crate::editor::document::Document;
use crate::services::Services;
use crate::ui::colorscheme::ColorScheme;
use crate::ui::theme::Theme;
use controller::actions::{Action, Mode};

pub struct Editor {
    pub mode: Mode,

    // settings
    pub use_colorscheme: bool,
    pub wrap: bool,
    pub syntax: bool,
    pub tree_sitter: bool,
    pub show_line_numbers: bool,
    pub fold: bool,
    pub fold_multiline_only: bool,
    // state
    pub should_redraw: bool,
    pub should_sync: bool,

    pub theme: Theme,
    pub colorscheme: ColorScheme,

    pub buffer_manager: BufferManager,

    pub services: Services,

    pub last_action: Action,
}

impl Editor {
    pub fn set_tree_sitter_enabled(&mut self, enabled: bool) {
        self.tree_sitter = enabled;
        if !enabled {
            for buffer in &mut self.buffer_manager.buffers {
                buffer
                    .latest_parse_task_id
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                buffer.syntax_tree = None;
            }
        }
    }

    pub fn apply_active_action(&mut self, action: &controller::actions::Action) {
        let active_idx = self.buffer_manager.active_idx;
        let mut document = std::mem::replace(
            &mut self.buffer_manager.buffers[active_idx].doc,
            Document::new("").unwrap(),
        );
        document.apply_action(action, self);
        self.buffer_manager.buffers[active_idx].doc = document;
    }

    pub fn apply_command_action(&mut self, action: &controller::actions::Action) {
        // let mut command =
        //     std::mem::replace(&mut self.controller.command.cmd, Document::new("").unwrap());
        // command.apply_action(action, self);
        // self.controller.command.cmd = command;
    }

    pub fn new(file_paths: Vec<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut buffer_manager = BufferManager::new();
        let mut next_id = 0;
        for path in file_paths {
            buffer_manager.add_buffer(TextBuffer::new(next_id, &path)?);
            next_id += 1;
        }

        if buffer_manager.buffers.is_empty() {
            buffer_manager.add_buffer(TextBuffer::new(next_id, "")?);
        }

        let theme = Theme::new("base16-ocean.dark");
        let colorscheme = ColorScheme::load_default();
        let services = Services::new();

        Ok(Self {
            buffer_manager,
            mode: Mode::Normal,
            theme,
            colorscheme,
            use_colorscheme: true,
            wrap: false,
            syntax: true,
            tree_sitter: true,
            show_line_numbers: true,
            fold: true,
            fold_multiline_only: true,
            should_redraw: true,
            should_sync: true,
            services,
            last_action: Action::NoOp,
        })
    }
}
