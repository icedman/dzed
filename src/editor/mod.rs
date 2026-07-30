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
    pub fn set_tree_sitter_enabled(&mut self, ui: &mut crate::ui::Ui, buffer_manager: &mut BufferManager, enabled: bool) {
        self.tree_sitter = enabled;
        if !enabled {
            for window in ui.windows.values_mut() {
                if let Some(ref mut doc) = window.doc {
                    doc.latest_parse_task_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
            }
            for buffer in &mut buffer_manager.buffers {
                buffer.syntax_tree = None;
            }
        }
    }

    pub fn apply_active_action(
        &mut self,
        ui: &mut crate::ui::Ui,
        buffer_manager: &mut BufferManager,
        action: &controller::actions::Action,
    ) {
        let active_win_id = ui.focused_window_id.unwrap();
        let window = ui.windows.get_mut(&active_win_id).unwrap();
        let doc = window.doc.as_mut().unwrap();
        let buffer_id = window.buffer_id.unwrap();
        
        let text_buffer = buffer_manager.get_by_id_mut(buffer_id).unwrap();
        doc.apply_action(
            &mut text_buffer.buffer,
            action,
            self,
            text_buffer.syntax_tree.as_ref(),
        );
        self.mode = doc.mode();
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
