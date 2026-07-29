pub mod colorscheme;
pub mod layout;
pub mod renderer;
pub mod theme;
pub mod view;
pub mod widgets;
pub mod window;

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
}

impl Ui {
    pub fn new() -> Self {
        let layout = layout::LayoutNode::Split {
            direction: layout::SplitDirection::Vertical,
            constraints: vec![
                layout::SizeConstraint::Fixed(2),        // Tabs (1 row)
                layout::SizeConstraint::Percentage(1.0), // Editor
                layout::SizeConstraint::Fixed(3),        // Statusbar (1 row)
            ],
            children: vec![
                layout::LayoutNode::Leaf { window_id: 1 }, // Tabs
                layout::LayoutNode::Leaf { window_id: 0 }, // Editor
                layout::LayoutNode::Leaf { window_id: 2 }, // Statusbar
            ],
        };

        let mut windows = HashMap::new();

        // Create initial default window
        let main_win_id = 0;
        let mut main_win = window::Window::new(main_win_id, "Editor".to_string());
        main_win.draw_border = true;
        windows.insert(main_win_id, main_win);

        // Create tabs window
        let mut tabs_win = window::Window::new(1, "Tabs".to_string());
        tabs_win.draw_border = true;
        windows.insert(1, tabs_win);

        // Create status bar window
        let mut statusbar_win = window::Window::new(2, "Status Bar".to_string());
        statusbar_win.draw_border = true;
        statusbar_win.set_view(Box::new(widgets::statusbar::StatusBarView {}));
        windows.insert(2, statusbar_win);

        Self {
            layout,
            screen_rows: 0,
            screen_cols: 0,
            last_parent_rect: None,
            cached_layouts: Vec::new(),
            windows,
            focused_window_id: None,
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

    pub fn update(&mut self, editor: &mut Editor) -> Result<(), Box<dyn std::error::Error>> {
        // Handle terminal resize.
        let (screen_cols, screen_rows) = {
            let (cols, rows) = crossterm::terminal::size().unwrap();
            (cols as i32, rows as i32)
        };

        // Recompute layout if needed.
        // Update window rects.
        self.layout(screen_cols as u32, screen_rows as u32);

        // Update cursor blinking.
        // Update animations.

        Ok(())
    }

    pub fn draw<W: Write>(
        &mut self,
        stdout: &mut W,
        editor: &mut Editor,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let computed = &self.cached_layouts;
        for &(win_id, rect) in computed {
            if let Some(win) = self.windows.get_mut(&win_id) {
                win.draw(stdout, rect, editor)?;
            }
        }
        Ok(())
    }
}
