use crate::controller::ControllerResult;
use crate::controller::actions;
use crate::controller::ex;
use crate::controller::exmap;
use crate::editor::Editor;
use crate::editor::buffers::TextBuffer;
use crate::ui::colorscheme;

pub struct Command {
    pub cmd: String,
    pub exmap: exmap::ExMap,
}

impl Command {
    pub fn new() -> Self {
        Self {
            cmd: String::new(),
            exmap: exmap::ExMap::new(),
        }
    }

    pub fn set(&mut self, text: &str) {
        self.cmd = text.to_string();
    }

    pub fn clear(&mut self) {
        self.cmd.clear();
    }

    pub fn get_text(&self) -> String {
        self.cmd.clone()
    }

    pub fn try_resolve_action(
        &self,
        cmd: &ex::ExCommand,
        _editor: &mut Editor,
        _buffer_manager: &mut crate::editor::buffers::BufferManager,
    ) -> actions::Action {
        if let Some(range) = &cmd.range {
            if let (Some(start), Some(end)) = (range.start_line, range.end_line) {
                match cmd.op {
                    ex::Ex::Delete => {
                        return actions::Action::DeleteLines {
                            start_line: start,
                            end_line: end,
                        };
                    }
                    ex::Ex::Yank => {
                        return actions::Action::YankLines {
                            start_line: start,
                            end_line: end,
                        };
                    }
                    _ => {}
                }
            }
        }
        actions::Action::NoOp
    }

    pub fn ex(
        &mut self,
        ui: &mut crate::ui::Ui,
        editor: &mut Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
    ) -> Option<ControllerResult> {
        let cmd_text = self.get_text();
        if let Some(resolved) = self.exmap.try_resolve(&cmd_text) {
            let action = self.try_resolve_action(&resolved, editor, buffer_manager);
            if action != actions::Action::NoOp {
                return Some(ControllerResult::Action(action.clone()));
            }
            match resolved.op {
                ex::Ex::Set => {
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
                                "tree" => editor.set_tree_sitter_enabled(ui, buffer_manager, true),
                                "notree" => {
                                    editor.set_tree_sitter_enabled(ui, buffer_manager, false)
                                }
                                "treesitter" => {
                                    editor.set_tree_sitter_enabled(ui, buffer_manager, true)
                                }
                                "notreesitter" => {
                                    editor.set_tree_sitter_enabled(ui, buffer_manager, false)
                                }
                                _ => {}
                            }
                        }
                    }
                    None
                }
                ex::Ex::Quit => Some(ControllerResult::Exit),
                ex::Ex::Colorschemes => {
                    let name = resolved
                        .arguments
                        .as_ref()
                        .and_then(|args| args.first())
                        .map(|s| s.as_str())
                        .unwrap_or("tokyonight");
                    let loaded = colorscheme::ColorScheme::get_by_name(name)
                        .unwrap_or_else(|| colorscheme::ColorScheme::load_default());
                    editor.colorscheme = loaded;
                    None
                }
                ex::Ex::Syntax => {
                    let arg = resolved
                        .arguments
                        .as_ref()
                        .and_then(|args| args.first())
                        .map(|s| s.as_str());
                    match arg {
                        Some("on") => editor.syntax = true,
                        Some("off") => editor.syntax = false,
                        _ => {}
                    }
                    None
                }
                ex::Ex::Bnext => {
                    if let Some(win) = ui.get_focused_window_mut() {
                        if let Some(current_id) = win.buffer_id {
                            if !buffer_manager.buffers.is_empty() {
                                if let Some(pos) = buffer_manager
                                    .buffers
                                    .iter()
                                    .position(|b| b.id == current_id)
                                {
                                    let next_idx = (pos + 1) % buffer_manager.buffers.len();
                                    let next_buf = &buffer_manager.buffers[next_idx];
                                    win.buffer_id = Some(next_buf.id);
                                    win.doc =
                                        Some(crate::editor::document::Document::new_with_buffer(
                                            next_buf.id,
                                            &next_buf.buffer,
                                            &next_buf.file_path,
                                        ));
                                }
                            }
                        }
                    }
                    None
                }
                ex::Ex::Bprev => {
                    if let Some(win) = ui.get_focused_window_mut() {
                        if let Some(current_id) = win.buffer_id {
                            if !buffer_manager.buffers.is_empty() {
                                if let Some(pos) = buffer_manager
                                    .buffers
                                    .iter()
                                    .position(|b| b.id == current_id)
                                {
                                    let prev_idx = if pos == 0 {
                                        buffer_manager.buffers.len() - 1
                                    } else {
                                        pos - 1
                                    };
                                    let prev_buf = &buffer_manager.buffers[prev_idx];
                                    win.buffer_id = Some(prev_buf.id);
                                    win.doc =
                                        Some(crate::editor::document::Document::new_with_buffer(
                                            prev_buf.id,
                                            &prev_buf.buffer,
                                            &prev_buf.file_path,
                                        ));
                                }
                            }
                        }
                    }
                    None
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Editor;
    use crate::editor::buffers::TextBuffer;
    use crate::editor::document::Document;
    use actions::Action;

    #[test]
    fn test_try_resolve_action() {
        let mut editor = Editor::new().unwrap();
        let mut buffer_manager = crate::editor::buffers::BufferManager::new();
        let cmd = Command::new();

        let resolved = cmd.exmap.try_resolve("1,10d").unwrap();
        let act = cmd.try_resolve_action(&resolved, &mut editor, &mut buffer_manager);
        assert_eq!(
            act,
            Action::DeleteLines {
                start_line: 1,
                end_line: 10
            }
        );

        let resolved2 = cmd.exmap.try_resolve("5y").unwrap();
        let act2 = cmd.try_resolve_action(&resolved2, &mut editor, &mut buffer_manager);
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
        let mut editor = Editor::new().unwrap();
        let mut buffer_manager = crate::editor::buffers::BufferManager::new();
        buffer_manager
            .add_buffer_for_path("temp_test_file1.txt")
            .unwrap();
        let mut cmd = Command::new();
        let main_win = crate::ui::WindowId::MainWindow as usize;

        cmd.set("set wrap");
        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert!(editor.wrap);

        cmd.set("set nowrap");
        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert!(!editor.wrap);

        cmd.set("set nonu");
        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert!(!editor.show_line_numbers);

        cmd.set("set nu");
        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert!(editor.show_line_numbers);

        cmd.set("set nofold");
        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert!(!editor.fold);

        cmd.set("set fold");
        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert!(editor.fold);

        cmd.set("set nofoldmultiline");
        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert!(!editor.fold_multiline_only);

        cmd.set("set foldmultiline");
        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert!(editor.fold_multiline_only);

        cmd.set("set notreesitter");
        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert!(!editor.tree_sitter);

        cmd.set("set treesitter");
        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert!(editor.tree_sitter);

        // Test colorschemes command and aliases
        cmd.set("colorschemes catppuccin");
        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert_eq!(editor.colorscheme.metadata.name, "catppuccin-mocha");

        cmd.set("colorscheme kanagawa");
        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert_eq!(editor.colorscheme.metadata.name, "kanagawa");

        cmd.set("colo catppuccin");
        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert_eq!(editor.colorscheme.metadata.name, "catppuccin-mocha");

        cmd.set("colorscheme unknown_colorscheme");
        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert_eq!(editor.colorscheme.metadata.name, "catppuccin-mocha");

        // Test syntax command
        cmd.set("syntax off");
        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert!(!editor.syntax);

        cmd.set("syn on");
        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert!(editor.syntax);

        // Test bnext / bprev commands
        buffer_manager
            .add_buffer_for_path("temp_test_file2.txt")
            .unwrap();

        let mut ui = crate::ui::Ui::new();
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let active_buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(active_buf.id);
            win.doc = Some(Document::new_with_buffer(
                active_buf.id,
                &active_buf.buffer,
                &active_buf.file_path,
            ));
        }

        cmd.set("bnext");
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert_eq!(ui.windows.get(&main_win).unwrap().buffer_id, Some(1));

        cmd.set("bprev");
        cmd.ex(&mut ui, &mut editor, &mut buffer_manager);
        assert_eq!(ui.windows.get(&main_win).unwrap().buffer_id, Some(0));
    }

    #[test]
    fn test_command_dispatch() {
        let mut editor = Editor::new().unwrap();
        let mut buffer_manager = crate::editor::buffers::BufferManager::new();
        let active_buf = buffer_manager.add_buffer_for_path("").unwrap();
        let _active_buf_id = active_buf.id;

        let mut ui = crate::ui::Ui::new();

        let main_win = crate::ui::WindowId::MainWindow as usize;
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(buf.id);
            win.doc = Some(Document::new_with_buffer(
                buf.id,
                &buf.buffer,
                &buf.file_path,
            ));
        }

        let cmd_buf = buffer_manager.add_buffer_for_path("#command").unwrap();
        let cmd_id = cmd_buf.id;
        let cmd_file_path = cmd_buf.file_path.clone();
        let cmd_win = crate::ui::WindowId::CommandLine as usize;
        if let Some(win) = ui.windows.get_mut(&cmd_win) {
            let cmd_buffer = buffer_manager.find_by_path(&cmd_file_path).unwrap();
            win.buffer_id = Some(cmd_id);
            win.doc = Some(Document::new_with_buffer(
                cmd_id,
                &cmd_buffer.buffer,
                &cmd_buffer.file_path,
            ));
        }

        let mut controller = crate::controller::Controller::new();

        {
            let doc = ui.windows.get_mut(&cmd_win).unwrap().doc.as_mut().unwrap();
            let buf = buffer_manager.find_mut(doc).unwrap();
            buf.buffer.edit([(0..0, "set nu")]);
            doc.clear(&buf.buffer);
        }

        editor.mode = crate::controller::actions::Mode::Command;
        if let Some(win) = ui.windows.get_mut(&cmd_win) {
            let buf = &buffer_manager
                .find(win.doc.as_ref().unwrap())
                .unwrap()
                .buffer;
            win.doc
                .as_mut()
                .unwrap()
                .enter_mode(buf, crate::controller::actions::Mode::Command);
        }

        ui.focus_window(cmd_win);

        controller
            .pending_actions
            .push_back(Action::InsertNewLine { count: 1 });

        controller
            .dispatch_actions(&mut editor, &mut buffer_manager, &mut ui)
            .unwrap();

        assert!(editor.show_line_numbers);
    }

    #[test]
    fn test_command_history_cycling() {
        let mut editor = Editor::new().unwrap();
        let mut buffer_manager = crate::editor::buffers::BufferManager::new();
        let active_buf = buffer_manager.add_buffer_for_path("").unwrap();
        let _active_buf_id = active_buf.id;
        
        let mut ui = crate::ui::Ui::new();
        
        let main_win = crate::ui::WindowId::MainWindow as usize;
        if let Some(win) = ui.windows.get_mut(&main_win) {
            let buf = &buffer_manager.buffers[0];
            win.buffer_id = Some(buf.id);
            win.doc = Some(Document::new_with_buffer(
                buf.id,
                &buf.buffer,
                &buf.file_path,
            ));
        }
        
        let cmd_buf = buffer_manager.add_buffer_for_path("#command").unwrap();
        let cmd_id = cmd_buf.id;
        let cmd_file_path = cmd_buf.file_path.clone();
        let cmd_win = crate::ui::WindowId::CommandLine as usize;
        
        let cmd_controller = crate::controller::controllers::commandline::CommandLineController::new();
        
        if let Some(win) = ui.windows.get_mut(&cmd_win) {
            let cmd_buffer = buffer_manager.find_by_path(&cmd_file_path).unwrap();
            win.buffer_id = Some(cmd_id);
            win.doc = Some(Document::new_with_buffer(
                cmd_id,
                &cmd_buffer.buffer,
                &cmd_buffer.file_path,
            ));
            win.controller = Some(Box::new(cmd_controller));
        }

        let mut controller = crate::controller::Controller::new();
        
        {
            let doc = ui.windows.get_mut(&cmd_win).unwrap().doc.as_mut().unwrap();
            let buf = buffer_manager.find_mut(doc).unwrap();
            buf.buffer.edit([(0..0, "set nu")]);
            doc.clear(&buf.buffer);
        }

        ui.focus_window(cmd_win);
        editor.mode = crate::controller::actions::Mode::Command;

        controller.pending_actions.push_back(Action::InsertNewLine { count: 1 });
        controller.dispatch_actions(&mut editor, &mut buffer_manager, &mut ui).unwrap();
        
        assert!(editor.show_line_numbers);

        {
            let doc = ui.windows.get_mut(&cmd_win).unwrap().doc.as_mut().unwrap();
            let buf = buffer_manager.find_mut(doc).unwrap();
            buf.buffer.edit([(0..0, "set nonu")]);
            doc.clear(&buf.buffer);
        }

        ui.focus_window(cmd_win);
        editor.mode = crate::controller::actions::Mode::Command;

        controller.pending_actions.push_back(Action::InsertNewLine { count: 1 });
        controller.dispatch_actions(&mut editor, &mut buffer_manager, &mut ui).unwrap();
        assert!(!editor.show_line_numbers);

        ui.focus_window(cmd_win);
        editor.mode = crate::controller::actions::Mode::Command;

        controller.pending_actions.push_back(Action::MoveUp { select: false, count: 1 });
        controller.dispatch_actions(&mut editor, &mut buffer_manager, &mut ui).unwrap();

        {
            let doc = ui.windows.get(&cmd_win).unwrap().doc.as_ref().unwrap();
            let buf = buffer_manager.find(doc).unwrap();
            assert_eq!(buf.buffer.snapshot().text(), "set nonu");
        }

        controller.pending_actions.push_back(Action::MoveUp { select: false, count: 1 });
        controller.dispatch_actions(&mut editor, &mut buffer_manager, &mut ui).unwrap();

        {
            let doc = ui.windows.get(&cmd_win).unwrap().doc.as_ref().unwrap();
            let buf = buffer_manager.find(doc).unwrap();
            assert_eq!(buf.buffer.snapshot().text(), "set nu");
        }

        controller.pending_actions.push_back(Action::MoveDown { select: false, count: 1 });
        controller.dispatch_actions(&mut editor, &mut buffer_manager, &mut ui).unwrap();

        {
            let doc = ui.windows.get(&cmd_win).unwrap().doc.as_ref().unwrap();
            let buf = buffer_manager.find(doc).unwrap();
            assert_eq!(buf.buffer.snapshot().text(), "set nonu");
        }

        controller.pending_actions.push_back(Action::MoveDown { select: false, count: 1 });
        controller.dispatch_actions(&mut editor, &mut buffer_manager, &mut ui).unwrap();

        {
            let doc = ui.windows.get(&cmd_win).unwrap().doc.as_ref().unwrap();
            let buf = buffer_manager.find(doc).unwrap();
            assert_eq!(buf.buffer.snapshot().text(), "");
        }
    }
}
