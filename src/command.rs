use crate::document::Document;
use onig::Regex;

pub struct Command {
    pub cmd: Document,
    pub command_history: Vec<String>,
    pub search_history: Vec<String>,
    pub history_idx: usize,
    pub search: bool,
    pub pattern: bool,
    pub search_text: String,
    pub regex_string: String,
    pub regex: Option<Regex>,
}

impl Command {
    pub fn new() -> Self {
        Self {
            cmd: Document::new_with_text(""),
            command_history: Vec::new(),
            search_history: Vec::new(),
            history_idx: 0,
            search: false,
            pattern: false,
            search_text: String::new(),
            regex_string: String::new(),
            regex: None,
        }
    }

    pub fn push(&mut self, text: &str) {
        let mut current = self.get_text();
        current.push_str(text);
        self.cmd = Document::new_with_text(&current);
    }

    pub fn set(&mut self, text: &str) {
        self.cmd = Document::new_with_text(text);
    }

    pub fn clear(&mut self) {
        self.cmd = Document::new_with_text("");
    }

    pub fn get_text(&self) -> String {
        let rope = self.cmd.buffer().as_rope();
        rope.chunks_in_range(0..rope.len()).collect()
    }

    pub fn ex(&mut self, _editor: &mut crate::editor::Editor) -> Option<ExResult> {
        let cmd_text = self.get_text();
        let map = crate::exmap::ExMap::new();
        if let Some(resolved) = map.try_resolve(&cmd_text) {
            match resolved.op {
                crate::ex::Ex::Quit => Some(ExResult::Exit),
                _ => None,
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExResult {
    Exit,
}
