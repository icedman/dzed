use crate::actions::Mode;
use crate::display::display_map::DisplayMap;
use crate::document::Document;
use crate::highlight::Highlights;
use crate::theme::Theme;
use onig::Regex;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

pub struct EditorBuffer {
    pub id: usize,
    pub file_path: String,
    pub doc: Document,
    pub display_map: DisplayMap,
    pub hl: Highlights,
    pub latest_hl_task_id: Arc<AtomicU64>,
    pub latest_wrap_task_id: Arc<AtomicU64>,
    pub latest_parse_task_id: Arc<AtomicU64>,
    pub current_hl_task_id: u64,
    pub current_wrap_task_id: u64,
    pub current_parse_task_id: u64,
    pub grammar: Option<crate::treesitter::grammars::Grammar>,
    pub syntax_tree: Option<crate::treesitter::SyntaxTree>,
}

impl EditorBuffer {
    pub fn new(id: usize, file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let doc = Document::new(file_path)?;
        let hl = Highlights::new(file_path);
        let display_map = DisplayMap::new(doc.buffer().snapshot().clone(), None);
        let grammar = crate::treesitter::grammars::Grammar::from_path(file_path);
        Ok(Self {
            id,
            file_path: file_path.to_string(),
            doc,
            display_map,
            hl,
            latest_hl_task_id: Arc::new(AtomicU64::new(0)),
            latest_wrap_task_id: Arc::new(AtomicU64::new(0)),
            latest_parse_task_id: Arc::new(AtomicU64::new(0)),
            current_hl_task_id: 0,
            current_wrap_task_id: 0,
            current_parse_task_id: 0,
            grammar,
            syntax_tree: None,
        })
    }
}

pub struct BufferManager {
    pub buffers: Vec<EditorBuffer>,
    pub active_idx: usize,
}

impl BufferManager {
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
            active_idx: 0,
        }
    }

    pub fn add_buffer(&mut self, buffer: EditorBuffer) {
        self.buffers.push(buffer);
        self.active_idx = self.buffers.len() - 1;
    }

    pub fn active(&self) -> &EditorBuffer {
        &self.buffers[self.active_idx]
    }

    pub fn active_mut(&mut self) -> &mut EditorBuffer {
        &mut self.buffers[self.active_idx]
    }

    pub fn switch_next(&mut self) {
        if !self.buffers.is_empty() {
            self.active_idx = (self.active_idx + 1) % self.buffers.len();
        }
    }

    pub fn switch_prev(&mut self) {
        if !self.buffers.is_empty() {
            if self.active_idx == 0 {
                self.active_idx = self.buffers.len() - 1;
            } else {
                self.active_idx -= 1;
            }
        }
    }

    pub fn get_by_id(&self, id: usize) -> Option<&EditorBuffer> {
        self.buffers.iter().find(|b| b.id == id)
    }

    pub fn get_by_id_mut(&mut self, id: usize) -> Option<&mut EditorBuffer> {
        self.buffers.iter_mut().find(|b| b.id == id)
    }

    pub fn switch_by_id(&mut self, id: usize) {
        if let Some(idx) = self.buffers.iter().position(|b| b.id == id) {
            self.active_idx = idx;
        }
    }
}

pub struct Editor {
    pub buffer_manager: BufferManager,
    pub mode: Mode,

    // global settings
    pub theme: Theme,
    pub wrap: bool,
    pub syntax: bool,
    pub tree_sitter: bool,
    pub show_line_numbers: bool,
    pub fold: bool,
    pub fold_multiline_only: bool,
    pub screen_rows: i32,
    pub screen_cols: i32,

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
            buffer_manager.add_buffer(EditorBuffer::new(next_id, &path)?);
            next_id += 1;
        }

        if buffer_manager.buffers.is_empty() {
            buffer_manager.add_buffer(EditorBuffer::new(next_id, "")?);
        }

        // let theme = Theme::new("base16-ocean.dark");
        let theme = Theme::new("test/themes/Dracula.tmTheme");
        let bg_worker = crate::background::BackgroundWorker::new();

        let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));

        Ok(Self {
            buffer_manager,
            command: crate::command::Command::new(),
            mode: Mode::Normal,
            theme,
            wrap: false,
            syntax: true,
            tree_sitter: true,
            show_line_numbers: true,
            fold: true,
            fold_multiline_only: true,
            screen_rows: rows as i32,
            screen_cols: cols as i32,
            bg_worker,
            clipboard: std::cell::RefCell::new(crate::clipboard::Clipboard::new()),
            keymap: crate::keymap::Keymap::new(),
            input: crate::input::VimInput::new(),
            macro_recorder: crate::macros::MacroRecorder::new(),
        })
    }
}
