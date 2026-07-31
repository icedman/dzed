pub mod colorscheme;
pub mod layout;
pub mod renderer;
pub mod views;
pub mod window;

pub use window::WindowId;

use crate::controller::controllers;
use crate::editor::Editor;
use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyEvent, MouseEventKind},
    execute,
    terminal::{Clear, ClearType},
};

use std::collections::HashMap;
use std::io::Write;

pub struct Ui {
    pub layout: layout::LayoutNode,
    pub screen_rows: u32,
    pub screen_cols: u32,
    pub last_parent_rect: Option<layout::Rect>,
    pub cached_layouts: Vec<(usize, layout::Rect)>,
    pub windows: HashMap<usize, window::Window>,
    pub focused_window_id: Option<usize>,
    next_window_id: usize,
}

impl Ui {
    pub fn new() -> Self {
        let layout = layout::LayoutNode::Split {
            direction: layout::SplitDirection::Vertical,
            constraints: vec![
                layout::SizeConstraint::Fixed(1),        // Tabs (1 row)
                layout::SizeConstraint::Percentage(1.0), // Editor
                layout::SizeConstraint::Fixed(1),        // Statusbar (1 row)
                layout::SizeConstraint::Fixed(1),        // CommandLine (1 row)
            ],
            children: vec![
                layout::LayoutNode::Leaf { window_id: WindowId::Tabs as usize }, // Tabs
                layout::LayoutNode::Leaf { window_id: WindowId::MainWindow as usize }, // Editor
                layout::LayoutNode::Leaf { window_id: WindowId::StatusBar as usize }, // Statusbar
                layout::LayoutNode::Leaf { window_id: WindowId::CommandLine as usize }, // CommandLine
            ],
        };

        let mut windows = HashMap::new();

        // Create initial default window
        let main_win_id = WindowId::MainWindow as usize;
        let mut main_win = window::Window::new(main_win_id, "Editor".to_string());
        main_win.set_view(Box::new(views::textview::TextView::new()));
        main_win.set_controller(Box::new(controllers::textview::TextViewController::new()));
        main_win.draw_border = true;
        windows.insert(main_win_id, main_win);

        // Create tabs window
        let tabs_win_id = WindowId::Tabs as usize;
        let mut tabs_win = window::Window::new(tabs_win_id, "Tabs".to_string());
        tabs_win.set_view(Box::new(views::tabs::TabsView {}));
        tabs_win.draw_border = false;
        windows.insert(tabs_win_id, tabs_win);

        // Create status bar window
        let statusbar_win_id = WindowId::StatusBar as usize;
        let mut statusbar_win = window::Window::new(statusbar_win_id, "Status Bar".to_string());
        statusbar_win.set_view(Box::new(views::statusbar::StatusBarView {}));
        statusbar_win.draw_border = false;
        windows.insert(statusbar_win_id, statusbar_win);

        // Create command bar window
        let commandline_win_id = WindowId::CommandLine as usize;
        let mut commandline_win = window::Window::new(commandline_win_id, "Command".to_string());
        commandline_win.set_view(Box::new(views::commandline::CommandLineView {}));
        commandline_win.draw_border = false;
        windows.insert(commandline_win_id, commandline_win);

        Self {
            layout,
            screen_rows: 0,
            screen_cols: 0,
            last_parent_rect: None,
            cached_layouts: Vec::new(),
            windows,
            focused_window_id: Some(main_win_id),
            next_window_id: 5,
        }
    }

    fn layout(&mut self, screen_cols: u32, screen_rows: u32) -> bool {
        if self.screen_cols == screen_cols && self.screen_rows == screen_rows {
            return false;
        }
        self.screen_rows = screen_rows;
        self.screen_cols = screen_cols;

        let parent_rect = layout::Rect {
            x: 0,
            y: 0,
            width: screen_cols as u16,
            height: screen_rows as u16,
        };
        if self.last_parent_rect != Some(parent_rect) {
            self.cached_layouts = self.layout.compute_layout(parent_rect);
            self.last_parent_rect = Some(parent_rect);
        }

        return true;
    }

    pub fn create_window(&mut self, id: usize) -> &mut window::Window {
        let actual_id = if id == WindowId::Any as usize {
            let nid = self.next_window_id;
            self.next_window_id += 1;
            nid
        } else {
            id
        };
        let win = window::Window::new(actual_id, String::new());
        self.windows.insert(actual_id, win);
        self.windows.get_mut(&actual_id).unwrap()
    }

    pub fn set_focused_window(&mut self, window_id: usize) {
        self.focused_window_id = Some(window_id);
    }

    pub fn get_focused_window(&self) -> Option<&window::Window> {
        self.focused_window_id.and_then(|id| self.windows.get(&id))
    }

    pub fn get_focused_window_mut(&mut self) -> Option<&mut window::Window> {
        self.focused_window_id
            .and_then(|id| self.windows.get_mut(&id))
    }

    pub fn update(&mut self, editor: &mut Editor, buffer_manager: &mut crate::editor::buffers::BufferManager) -> Result<(), Box<dyn std::error::Error>> {
        // Handle terminal resize.
        let (screen_cols, screen_rows) = {
            let (cols, rows) = crossterm::terminal::size().unwrap();
            (cols as i32, rows as i32)
        };

        // Recompute layout if needed.
        // Update window rects.
        if self.layout(screen_cols as u32, screen_rows as u32) {
            for window in self.windows.values_mut() {
                if let Some(ref mut doc) = window.doc {
                    doc.should_sync = true;
                }
            }
            editor.should_redraw = true;
        }

        let computed = self.cached_layouts.clone();
        for &(window_id, rect) in &computed {
            if let Some(window) = self.windows.get_mut(&window_id) {
                let mut controller = window.controller.take();
                if let Some(ref mut c) = controller {
                    c.update(editor, buffer_manager, self, window_id, rect)?;
                }
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.controller = controller;
                }
            }
        }

        // Update cursor blinking.
        // Update animations.

        Ok(())
    }

    pub fn draw<W: Write>(
        &mut self,
        stdout: &mut W,
        editor: &mut Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Temporarily take the active document to bypass borrow checker
        let mut active_doc = self.focused_window_id
            .and_then(|id| self.windows.get_mut(&id))
            .and_then(|win| win.doc.take());

        let computed = self.cached_layouts.clone();
        for &(win_id, rect) in &computed {
            if let Some(mut win) = self.windows.remove(&win_id) {
                if Some(win_id) == self.focused_window_id {
                    win.doc = active_doc.take();
                }

                win.draw(stdout, rect, editor, buffer_manager, active_doc.as_ref(), self)?;

                if Some(win_id) == self.focused_window_id {
                    active_doc = win.doc.take();
                }

                self.windows.insert(win_id, win);
            }
        }

        // Put it back permanently
        if let Some(id) = self.focused_window_id {
            if let Some(win) = self.windows.get_mut(&id) {
                win.doc = active_doc;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_window() {
        let mut ui = Ui::new();
        let win_id = 999;
        assert!(ui.windows.get(&win_id).is_none());
        
        {
            let win = ui.create_window(win_id);
            assert_eq!(win.id, win_id);
            win.title = "Test Window".to_string();
        }
        
        let win = ui.windows.get(&win_id).unwrap();
        assert_eq!(win.id, win_id);
        assert_eq!(win.title, "Test Window");
    }
}

