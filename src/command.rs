use crate::document::Document;
use crate::ex;
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

    pub fn try_resolve_action(
        &self,
        cmd: &crate::ex::ExCommand,
        _editor: &mut crate::editor::Editor,
    ) -> crate::actions::Action {
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
                crate::ex::Ex::Set => {
                    if let Some(args) = &resolved.arguments {
                        for arg in args {
                            match arg.as_str() {
                                "wrap" => editor.wrap = true,
                                "nowrap" => editor.wrap = false,
                                "nu" => editor.show_line_numbers = true,
                                "nonu" => editor.show_line_numbers = false,
                                "number" => editor.show_line_numbers = true,
                                "nonumber" => editor.show_line_numbers = false,
                                "fold" => editor.fold = true,
                                "nofold" => editor.fold = false,
                                "foldmultiline" => editor.fold_multiline_only = true,
                                "nofoldmultiline" => editor.fold_multiline_only = false,
                                "tree" => editor.set_tree_sitter_enabled(true),
                                "notree" => editor.set_tree_sitter_enabled(false),
                                "treesitter" => editor.set_tree_sitter_enabled(true),
                                "notreesitter" => editor.set_tree_sitter_enabled(false),
                                _ => {}
                            }
                        }
                    }
                    None
                }
                crate::ex::Ex::Quit => Some(ExResult::Exit),
                crate::ex::Ex::Colorschemes => {
                    let name = resolved.arguments.as_ref()
                        .and_then(|args| args.first())
                        .map(|s| s.as_str())
                        .unwrap_or("tokyonight");
                    let loaded = crate::colorscheme::ColorScheme::get_by_name(name)
                        .unwrap_or_else(|| crate::colorscheme::ColorScheme::load_default());
                    editor.colorscheme = loaded;
                    None
                }
                crate::ex::Ex::Syntax => {
                    let arg = resolved.arguments.as_ref()
                        .and_then(|args| args.first())
                        .map(|s| s.as_str());
                    match arg {
                        Some("on") => editor.syntax = true,
                        Some("off") => editor.syntax = false,
                        _ => {}
                    }
                    None
                }
                crate::ex::Ex::Bnext => {
                    editor.buffer_manager.switch_next();
                    None
                }
                crate::ex::Ex::Bprev => {
                    editor.buffer_manager.switch_prev();
                    None
                }
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
    use crate::actions::Action;
    use crate::editor::Editor;

    #[test]
    fn test_try_resolve_action() {
        let mut editor = Editor::new(Vec::new()).unwrap();
        let cmd = Command::new();

        let resolved = cmd.exmap.try_resolve("1,10d").unwrap();
        let act = cmd.try_resolve_action(&resolved, &mut editor);
        assert_eq!(
            act,
            Action::DeleteLines {
                start_line: 1,
                end_line: 10
            }
        );

        let resolved2 = cmd.exmap.try_resolve("5y").unwrap();
        let act2 = cmd.try_resolve_action(&resolved2, &mut editor);
        assert_eq!(
            act2,
            Action::YankLines {
                start_line: 5,
                end_line: 5
            }
        );
    }

    #[test]
    fn test_ex_set() {
        let mut editor = Editor::new(Vec::new()).unwrap();
        let mut cmd = Command::new();

        cmd.set("set wrap");
        cmd.ex(&mut editor);
        assert!(editor.wrap);

        cmd.set("set nowrap");
        cmd.ex(&mut editor);
        assert!(!editor.wrap);

        cmd.set("set nonu");
        cmd.ex(&mut editor);
        assert!(!editor.show_line_numbers);

        cmd.set("set nu");
        cmd.ex(&mut editor);
        assert!(editor.show_line_numbers);

        cmd.set("set nofold");
        cmd.ex(&mut editor);
        assert!(!editor.fold);

        cmd.set("set fold");
        cmd.ex(&mut editor);
        assert!(editor.fold);

        cmd.set("set nofoldmultiline");
        cmd.ex(&mut editor);
        assert!(!editor.fold_multiline_only);

        cmd.set("set foldmultiline");
        cmd.ex(&mut editor);
        assert!(editor.fold_multiline_only);

        cmd.set("set notreesitter");
        cmd.ex(&mut editor);
        assert!(!editor.tree_sitter);

        cmd.set("set treesitter");
        cmd.ex(&mut editor);
        assert!(editor.tree_sitter);

        // Test colorschemes command and aliases
        cmd.set("colorschemes catppuccin");
        cmd.ex(&mut editor);
        assert_eq!(editor.colorscheme.metadata.name, "catppuccin-mocha");

        cmd.set("colorscheme kanagawa");
        cmd.ex(&mut editor);
        assert_eq!(editor.colorscheme.metadata.name, "kanagawa");

        cmd.set("colo catppuccin");
        cmd.ex(&mut editor);
        assert_eq!(editor.colorscheme.metadata.name, "catppuccin-mocha");

        cmd.set("colorscheme unknown_colorscheme");
        cmd.ex(&mut editor);
        assert_eq!(editor.colorscheme.metadata.name, "tokyonight-moon");

        // Test syntax command
        cmd.set("syntax off");
        cmd.ex(&mut editor);
        assert!(!editor.syntax);

        cmd.set("syn on");
        cmd.ex(&mut editor);
        assert!(editor.syntax);

        // Test bnext / bprev commands
        let buf2 = crate::editor::EditorBuffer::new(99, "temp_test_file2.txt").unwrap();
        editor.buffer_manager.add_buffer(buf2);
        // Switch active index to first buffer (index 0)
        editor.buffer_manager.active_idx = 0;
        assert_eq!(editor.buffer_manager.active_idx, 0);

        cmd.set("bnext");
        cmd.ex(&mut editor);
        assert_eq!(editor.buffer_manager.active_idx, 1);

        cmd.set("bprev");
        cmd.ex(&mut editor);
        assert_eq!(editor.buffer_manager.active_idx, 0);
    }
}
