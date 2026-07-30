use crate::editor::document::Document;
use crate::services::{self};
use text::Buffer;

pub struct TextBuffer {
    pub id: usize,
    pub file_path: String,
    pub buffer: Buffer,
    pub doc: Document,
    pub grammar: Option<services::treesitter::grammars::Grammar>,
    pub syntax_tree: Option<services::treesitter::SyntaxTree>,
}

impl TextBuffer {
    pub fn new(id: usize, file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = if std::path::Path::new(file_path).exists() {
            match std::fs::read_to_string(file_path) {
                Ok(s) => s,
                Err(_) => "File not found".to_string(),
            }
        } else {
            "".to_string()
        };
        let buffer = Buffer::new(
            clock::ReplicaId::default(),
            text::BufferId::new(1).unwrap(),
            contents,
        );
        let doc = Document::new_with_buffer(id, &buffer, file_path);
        let grammar = services::treesitter::grammars::Grammar::from_path(file_path);
        Ok(Self {
            id,
            file_path: file_path.to_string(),
            buffer,
            doc,
            grammar,
            syntax_tree: None,
        })
    }

    pub fn new_with_text(contents: &str) -> Self {
        let buffer = Buffer::new(
            clock::ReplicaId::default(),
            text::BufferId::new(1).unwrap(),
            contents.to_string(),
        );
        let doc = Document::new_with_buffer(0, &buffer, "");
        Self {
            id: 0,
            file_path: "".to_string(),
            buffer,
            doc,
            grammar: None,
            syntax_tree: None,
        }
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

    pub fn find(&self, doc: &Document) -> Option<&TextBuffer> {
        self.buffers.iter().find(|b| b.id == doc.id)
    }
}
