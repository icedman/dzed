pub mod buffers;
pub mod display;
pub mod document;
pub mod selections;

use crate::controller::{self};
use crate::editor::buffers::BufferManager;
use crate::editor::document::Document;
use crate::services::Services;
use crate::ui::colorscheme::ColorScheme;

use controller::actions::{Action, Mode};

pub struct Editor {
    pub mode: Mode,

    // settings
    pub wrap: bool,
    pub syntax: bool,
    pub tree_sitter: bool,
    pub show_line_numbers: bool,
    pub fold: bool,
    pub fold_multiline_only: bool,
    // state
    pub should_redraw: bool,

    pub colorscheme: ColorScheme,

    pub services: Services,

    pub textmate_highlights: bool,
    pub treesitter_highlights: bool,
    pub map_scope_to_scheme: bool,
    pub last_action: Action,
    pub pending_keys: String,
    pub search_pattern: String,
    pub search_regex: Option<onig::Regex>,
}

impl Editor {
    pub fn set_tree_sitter_enabled(
        &mut self,
        ui: &mut crate::ui::Ui,
        buffer_manager: &mut BufferManager,
        enabled: bool,
    ) {
        self.tree_sitter = enabled;
        if !enabled {
            for window in ui.windows.values_mut() {
                if let Some(ref mut doc) = window.doc {
                    doc.latest_parse_task_id
                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
        let text_buffer = buffer_manager.find_mut(doc).unwrap();
        doc.apply_action(
            &mut text_buffer.buffer,
            action,
            self,
            text_buffer.syntax_tree.as_ref(),
        );
        self.mode = doc.mode();
    }

    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let colorscheme = ColorScheme::load_default();
        let services = Services::new();

        Ok(Self {
            mode: Mode::Normal,
            colorscheme,
            wrap: false,
            syntax: true,
            tree_sitter: true,
            show_line_numbers: true,
            fold: true,
            fold_multiline_only: false,
            should_redraw: true,
            services,
            textmate_highlights: true,
            treesitter_highlights: false,
            map_scope_to_scheme: true,
            last_action: Action::NoOp,
            pending_keys: String::new(),
            search_pattern: String::new(),
            search_regex: None,
        })
     }

    pub fn set_pattern(&mut self, pattern: String) {
        if self.search_pattern == pattern && self.search_regex.is_some() {
            return;
        }
        self.search_regex = if pattern.is_empty() {
            None
        } else {
            onig::Regex::new(&pattern).ok()
        };
        self.search_pattern = pattern;
    }
}
