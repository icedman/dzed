pub mod buffers;
pub mod display;
pub mod document;
pub mod selections;

use crate::controller::{self};
use crate::editor::buffers::BufferManager;
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

    pub theme: Theme,
    pub colorscheme: ColorScheme,

    pub services: Services,

    pub last_action: Action,
}

impl Editor {
    pub fn set_tree_sitter_enabled(&mut self, buffer_manager: &mut BufferManager, enabled: bool) {
        self.tree_sitter = enabled;
        if !enabled {
            for buffer in &mut buffer_manager.buffers {
                buffer
                    .doc
                    .latest_parse_task_id
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                buffer.syntax_tree = None;
            }
        }
    }

    pub fn apply_active_action(
        &mut self,
        buffer_manager: &mut BufferManager,
        action: &controller::actions::Action,
    ) {
        let active_idx = buffer_manager.active_idx;
        let dummy_buffer = text::Buffer::new(clock::ReplicaId::default(), text::BufferId::new(1).unwrap(), "".to_string());
        let mut document = std::mem::replace(
            &mut buffer_manager.buffers[active_idx].doc,
            Document::new_with_buffer(0, &dummy_buffer, ""),
        );
        let mut active_buffer = std::mem::replace(
            &mut buffer_manager.buffers[active_idx].buffer,
            dummy_buffer,
        );
        document.apply_action(&mut active_buffer, action, self, buffer_manager);
        self.mode = document.mode();
        buffer_manager.buffers[active_idx].buffer = active_buffer;
        buffer_manager.buffers[active_idx].doc = document;
    }

    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let theme = Theme::new("base16-ocean.dark");
        let colorscheme = ColorScheme::load_default();
        let services = Services::new();

        Ok(Self {
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
            services,
            last_action: Action::NoOp,
        })
    }
}
