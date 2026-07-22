use crate::actions::Mode;
use crate::display::display_map::DisplayMap;
use crate::document::Document;
use crate::highlight::Highlights;
use crate::theme::Theme;
use onig::Regex;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

pub struct EditorBuffer {
    pub file_path: String,
    pub doc: Document,
    pub display_map: DisplayMap,
    pub hl: Highlights,
    pub dirty_hl: bool,
    pub latest_hl_task_id: Arc<AtomicU64>,
    pub latest_wrap_task_id: Arc<AtomicU64>,
    pub current_hl_task_id: u64,
    pub current_wrap_task_id: u64,
}

impl EditorBuffer {
    pub fn new(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let doc = Document::new(file_path)?;
        let hl = Highlights::new(file_path);
        let display_map = DisplayMap::new(doc.buffer().snapshot().clone(), None);
        Ok(Self {
            file_path: file_path.to_string(),
            doc,
            display_map,
            hl,
            dirty_hl: true,
            latest_hl_task_id: Arc::new(AtomicU64::new(0)),
            latest_wrap_task_id: Arc::new(AtomicU64::new(0)),
            current_hl_task_id: 0,
            current_wrap_task_id: 0,
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
}

pub struct Editor {
    pub buffer_manager: BufferManager,
    pub cmd: Document,
    pub mode: Mode,
    pub theme: Theme,
    pub command_history: Vec<String>,
    pub search_history: Vec<String>,
    pub history_idx: usize,
    pub pending_cmd: String,

    pub search: bool,
    pub pattern: bool,
    pub search_text: String,
    pub regex_string: String,
    pub regex: Option<Regex>,

    pub wrap: bool,
    pub syntax: bool,
    pub show_line_numbers: bool,
    pub bg_worker: crate::background::BackgroundWorker,
    pub clipboard: crate::clipboard::Clipboard,
    pub keymap: crate::keymap::Keymap,
}

impl Editor {
    pub fn new(file_paths: Vec<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut buffer_manager = BufferManager::new();
        for path in file_paths {
            buffer_manager.add_buffer(EditorBuffer::new(&path)?);
        }

        if buffer_manager.buffers.is_empty() {
            buffer_manager.add_buffer(EditorBuffer::new("")?);
        }

        let cmd = Document::new("")?;
        // let theme = Theme::new("base16-ocean.dark");
        let theme = Theme::new("test/themes/Dracula.tmTheme");
        let bg_worker = crate::background::BackgroundWorker::new();

        Ok(Self {
            buffer_manager,
            cmd,
            command_history: Vec::new(),
            search_history: Vec::new(),
            history_idx: 0,
            pending_cmd: String::new(),
            search: false,
            pattern: false,
            search_text: "".to_string(),
            regex: None,
            regex_string: "".to_string(),
            mode: Mode::Normal,
            theme,
            wrap: false,
            syntax: true,
            show_line_numbers: false,
            bg_worker,
            clipboard: crate::clipboard::Clipboard::new(),
            keymap: crate::keymap::Keymap::default(),
        })
    }
}
