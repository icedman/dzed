use crate::actions::Mode;
use crate::buffers::BufferManager;
use crate::buffers::TextBuffer;
use crate::colorscheme::ColorScheme;
use crate::document::Document;
use crate::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppContext {
    // settings
    pub use_colorscheme: bool,
    pub wrap: bool,
    pub syntax: bool,
    pub tree_sitter: bool,
    pub show_line_numbers: bool,
    pub fold: bool,
    pub fold_multiline_only: bool,
    // state
    pub screen_rows: i32,
    pub screen_cols: i32,
    pub focused_window: Option<usize>,
    pub should_redraw: bool,
    pub should_sync: bool,
}

pub struct Editor {
    pub buffer_manager: BufferManager,
    pub mode: Mode,

    // global settings
    pub settings: AppContext,
    pub theme: Theme,
    pub colorscheme: ColorScheme,

    // commands
    pub command: crate::command::Command,

    // services
    pub bg_worker: crate::background::BackgroundWorker,
    pub clipboard: std::cell::RefCell<crate::clipboard::Clipboard>,

    // input
    pub keymap: crate::keymap::Keymap,
    pub input: crate::input::VimInput,
    pub macro_recorder: crate::macros::MacroRecorder,
}

impl Editor {
    pub fn set_tree_sitter_enabled(&mut self, enabled: bool) {
        self.settings.tree_sitter = enabled;
        if !enabled {
            for buffer in &mut self.buffer_manager.buffers {
                buffer
                    .latest_parse_task_id
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                buffer.syntax_tree = None;
            }
        }
    }

    pub fn apply_active_action(&mut self, action: &crate::actions::Action) {
        let active_idx = self.buffer_manager.active_idx;
        let mut document = std::mem::replace(
            &mut self.buffer_manager.buffers[active_idx].doc,
            Document::new("").unwrap(),
        );
        document.apply_action(action, self);
        self.buffer_manager.buffers[active_idx].doc = document;
    }

    pub fn apply_command_action(&mut self, action: &crate::actions::Action) {
        let mut command = std::mem::replace(&mut self.command.cmd, Document::new("").unwrap());
        command.apply_action(action, self);
        self.command.cmd = command;
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
        let bg_worker = crate::background::BackgroundWorker::new();

        let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));

        Ok(Self {
            buffer_manager,
            command: crate::command::Command::new(),
            mode: Mode::Normal,
            theme,
            colorscheme,
            settings: AppContext {
                use_colorscheme: true,
                wrap: false,
                syntax: true,
                tree_sitter: true,
                show_line_numbers: true,
                fold: true,
                fold_multiline_only: true,
                screen_rows: rows as i32,
                screen_cols: cols as i32,
                focused_window: None,
                should_redraw: true,
                should_sync: true,
            },
            bg_worker,
            clipboard: std::cell::RefCell::new(crate::clipboard::Clipboard::new()),
            keymap: crate::keymap::Keymap::new(),
            input: crate::input::VimInput::new(),
            macro_recorder: crate::macros::MacroRecorder::new(),
        })
    }
}
