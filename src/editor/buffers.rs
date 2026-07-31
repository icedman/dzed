use crate::editor::document::Document;
use crate::services::{self};
use text::Buffer;

pub struct TextBuffer {
    pub id: usize,
    pub file_path: String,
    pub buffer: Buffer,
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
        let grammar = services::treesitter::grammars::Grammar::from_path(file_path);
        Ok(Self {
            id,
            file_path: file_path.to_string(),
            buffer,
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
        Self {
            id: 0,
            file_path: "".to_string(),
            buffer,
            grammar: None,
            syntax_tree: None,
        }
    }
}

pub struct BufferManager {
    pub buffers: Vec<TextBuffer>,
}

impl BufferManager {
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
        }
    }

    pub fn add_buffer(&mut self, buffer: TextBuffer) {
        self.buffers.push(buffer);
    }

    pub fn find(&self, doc: &Document) -> Option<&TextBuffer> {
        self.buffers.iter().find(|b| b.id == doc.id)
    }

    pub fn find_mut(&mut self, doc: &Document) -> Option<&mut TextBuffer> {
        self.buffers.iter_mut().find(|b| b.id == doc.id)
    }

    pub fn find_by_path(&self, path: &str) -> Option<&TextBuffer> {
        self.buffers.iter().find(|b| b.file_path == path)
    }

    pub fn find_by_path_mut(&mut self, path: &str) -> Option<&mut TextBuffer> {
        self.buffers.iter_mut().find(|b| b.file_path == path)
    }

    pub fn add_buffer_for_path(&mut self, path: &str) -> Result<&mut TextBuffer, Box<dyn std::error::Error>> {
        if let Some(pos) = self.buffers.iter().position(|b| b.file_path == path) {
            return Ok(&mut self.buffers[pos]);
        }
        let next_id = self.buffers.iter().map(|b| b.id).max().map(|id| id + 1).unwrap_or(0);
        let new_buf = TextBuffer::new(next_id, path)?;
        self.buffers.push(new_buf);
        Ok(self.buffers.last_mut().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::document::Document;

    #[test]
    fn test_add_buffer_for_path() {
        let mut bm = BufferManager::new();
        let path = "test_file_path.txt";
        
        let id1 = {
            let buf1 = bm.add_buffer_for_path(path).unwrap();
            assert_eq!(buf1.file_path, path);
            buf1.id
        };

        // Try adding the same path again - should return the same buffer (same ID)
        let id2 = {
            let buf2 = bm.add_buffer_for_path(path).unwrap();
            buf2.id
        };
        assert_eq!(id2, id1);

        // Try adding a different path - should return a new buffer (new ID)
        let id3 = {
            let buf3 = bm.add_buffer_for_path("other_file_path.txt").unwrap();
            buf3.id
        };
        assert_ne!(id3, id1);
        assert_eq!(bm.buffers.len(), 2);
    }
}
