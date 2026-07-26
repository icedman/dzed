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
    pub exmap: crate::exmap::ExMap,
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
            exmap: crate::exmap::ExMap::new(),
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

    pub fn try_resolve_action(&self, cmd: &crate::ex::ExCommand, _editor: &mut crate::editor::Editor) -> crate::actions::Action {
        if let Some(range) = &cmd.range {
            if let (Some(start), Some(end)) = (range.start_line, range.end_line) {
                match cmd.op {
                    crate::ex::Ex::Delete => {
                        return crate::actions::Action::DeleteLines {
                            start_line: start,
                            end_line: end,
                        };
                    }
                    crate::ex::Ex::Yank => {
                        return crate::actions::Action::YankLines {
                            start_line: start,
                            end_line: end,
                        };
                    }
                    _ => {}
                }
            }
        }
        crate::actions::Action::NoOp
    }

    pub fn ex(&mut self, editor: &mut crate::editor::Editor) -> Option<ExResult> {
        let cmd_text = self.get_text();
        if let Some(resolved) = self.exmap.try_resolve(&cmd_text) {
            let action = self.try_resolve_action(&resolved, editor);
            if action != crate::actions::Action::NoOp {
                editor.apply_active_action(&action);
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Editor;
    use crate::actions::Action;

    #[test]
    fn test_try_resolve_action() {
        let mut editor = Editor::new(Vec::new()).unwrap();
        let cmd = Command::new();
        
        let resolved = cmd.exmap.try_resolve("1,10d").unwrap();
        let act = cmd.try_resolve_action(&resolved, &mut editor);
        assert_eq!(act, Action::DeleteLines { start_line: 1, end_line: 10 });

        let resolved2 = cmd.exmap.try_resolve("5y").unwrap();
        let act2 = cmd.try_resolve_action(&resolved2, &mut editor);
        assert_eq!(act2, Action::YankLines { start_line: 5, end_line: 5 });
    }
}
