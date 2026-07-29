use crate::controller::actions::Mode;
use crate::editor::display::display_map::DisplayMap;
use crate::editor::display::highlight::Highlights;
use crate::editor::document::Document;
use crate::services::{self};
use crate::ui::colorscheme::ColorScheme;
use crate::ui::theme::Theme;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

pub struct TextBuffer {
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
    pub grammar: Option<services::treesitter::grammars::Grammar>,
    pub syntax_tree: Option<services::treesitter::SyntaxTree>,
}

impl TextBuffer {
    pub fn new(id: usize, file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let doc = Document::new(file_path)?;
        let hl = Highlights::new(file_path);
        let display_map = DisplayMap::new(doc.buffer().snapshot().clone(), None);
        let grammar = services::treesitter::grammars::Grammar::from_path(file_path);
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
    pub buffers: Vec<TextBuffer>,
    pub active_idx: usize,
}

impl BufferManager {
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
            active_idx: 0,
        }
    }

    pub fn add_buffer(&mut self, buffer: TextBuffer) {
        self.buffers.push(buffer);
        self.active_idx = self.buffers.len() - 1;
    }

    pub fn active(&self) -> &TextBuffer {
        &self.buffers[self.active_idx]
    }

    pub fn active_mut(&mut self) -> &mut TextBuffer {
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

    pub fn get_by_id(&self, id: usize) -> Option<&TextBuffer> {
        self.buffers.iter().find(|b| b.id == id)
    }

    pub fn get_by_id_mut(&mut self, id: usize) -> Option<&mut TextBuffer> {
        self.buffers.iter_mut().find(|b| b.id == id)
    }

    pub fn switch_by_id(&mut self, id: usize) {
        if let Some(idx) = self.buffers.iter().position(|b| b.id == id) {
            self.active_idx = idx;
        }
    }
}
