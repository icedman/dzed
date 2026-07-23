pub mod editor;
pub mod layout;
pub mod statusbar;
pub mod tabs;
pub mod view;
pub mod window;

use crate::actions::Mode;
use crate::document::BufferText;
use crate::editor::Editor;
use crate::search::{TextSearch, compile};
use crate::theme::{ColorAdjust, ToCrossTerm};
use crossterm::{cursor::MoveTo, execute};
use std::io::{Stdout, Write};
use text::{Point, ToPoint};

/// The main UI class managing layouts, windows, and focus state.
pub struct Ui {
    pub layout: layout::LayoutNode,
    pub windows: std::collections::HashMap<usize, window::Window>,
    pub focused_window_id: Option<usize>,
    pub cached_layouts: Vec<(usize, layout::Rect)>,
    pub last_parent_rect: Option<layout::Rect>,
    pub dirty: bool,
    pub last_cursor_style: Option<crossterm::cursor::SetCursorStyle>,
}

impl Ui {
    pub fn new() -> Self {
        let mut windows = std::collections::HashMap::new();
        // Create initial default window
        let main_win_id = 0;
        let mut main_win = window::Window::new(main_win_id, "Editor".to_string());
        main_win.set_view(Box::new(editor::EditorView::new()));
        windows.insert(main_win_id, main_win);

        // Create tabs window
        let mut tabs_win = window::Window::new(1, "Tabs".to_string());
        tabs_win.set_view(Box::new(tabs::TabsView::new()));
        tabs_win.draw_border = false;
        windows.insert(1, tabs_win);

        // Create status bar window
        let mut statusbar_win = window::Window::new(2, "Status Bar".to_string());
        statusbar_win.set_view(Box::new(statusbar::StatusBarView::new()));
        statusbar_win.draw_border = false;
        windows.insert(2, statusbar_win);

        let layout = layout::LayoutNode::Split {
            direction: layout::SplitDirection::Vertical,
            constraints: vec![
                layout::SizeConstraint::Fixed(1),        // Tabs (1 row)
                layout::SizeConstraint::Percentage(1.0), // Editor
                layout::SizeConstraint::Fixed(1),        // Statusbar (1 row)
            ],
            children: vec![
                layout::LayoutNode::Leaf { window_id: 1 }, // Tabs
                layout::LayoutNode::Leaf { window_id: 0 }, // Editor
                layout::LayoutNode::Leaf { window_id: 2 }, // Statusbar
            ],
        };

        Self {
            layout,
            windows,
            focused_window_id: Some(main_win_id),
            cached_layouts: Vec::new(),
            last_parent_rect: None,
            dirty: true,
            last_cursor_style: None,
        }
    }

    /// Routes key/mouse events to the focused window's view if available.
    pub fn handle_event(
        &mut self,
        event: &crossterm::event::Event,
        editor: &mut Editor,
    ) -> Option<crate::input::HandleEvent> {
        if let Some(focused_id) = self.focused_window_id {
            if let Some(win) = self.windows.get_mut(&focused_id) {
                if let Some(ref mut view) = win.view {
                    return view.handle_event(event, editor);
                }
            }
        }
        None
    }

    /// Explicitly mark the layout as dirty to force a recalculation on next draw.
    pub fn set_dirty(&mut self) {
        self.dirty = true;
    }

    /// Renders the layout and all windows/components managed by this UI instance.
    pub fn draw(
        &mut self,
        stdout: &mut Stdout,
        editor: &mut Editor,
        screen_width: u16,
        screen_height: u16,
    ) -> std::io::Result<()> {
        execute!(stdout, crossterm::cursor::Hide)?;

        let parent_rect = layout::Rect {
            x: 0,
            y: 0,
            width: screen_width,
            height: screen_height,
        };

        if self.dirty || self.last_parent_rect != Some(parent_rect) {
            self.cached_layouts = self.layout.compute_layout(parent_rect);
            self.last_parent_rect = Some(parent_rect);
            self.dirty = false;
        }

        let computed = &self.cached_layouts;

        // Find editor window inner rect to position text & cursor correctly
        let mut editor_inner_rect = parent_rect;
        let mut editor_gutter_width = 0usize;

        for &(win_id, rect) in computed {
            if win_id == 0 {
                editor_inner_rect = layout::Rect {
                    x: rect.x + 1,
                    y: rect.y + 1,
                    width: rect.width.saturating_sub(2),
                    height: rect.height.saturating_sub(2),
                };

                let active_buffer = editor.buffer_manager.active_mut();
                let row_count = active_buffer.doc.buffer().row_count();
                editor_gutter_width = if editor.show_line_numbers {
                    2 + if row_count == 0 {
                        0
                    } else {
                        row_count.ilog10() as usize
                    }
                } else {
                    0
                };
                break;
            }
        }

        for &(win_id, rect) in computed {
            match win_id {
                0 | 1 => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.is_focused = Some(win_id) == self.focused_window_id;
                        win.draw(stdout, rect, editor)?;
                    }
                }
                2 => {
                    if editor.mode == Mode::Command {
                        render_command_line(stdout, editor, rect)?;
                    } else {
                        if let Some(win) = self.windows.get_mut(&win_id) {
                            win.is_focused = Some(win_id) == self.focused_window_id;
                            win.draw(stdout, rect, editor)?;
                        }
                    }
                }
                _ => {}
            }
        }

        // 3. Update cursor position
        let active_buffer = editor.buffer_manager.active();
        let display_snapshot = active_buffer.display_map.snapshot();

        let cursor = active_buffer.doc.selection();
        let cursor_point = cursor.head().to_point(active_buffer.doc.buffer());
        let display_cursor = active_buffer
            .display_map
            .snapshot()
            .point_to_display_point(cursor_point);

        let cursor_row = display_cursor.row() as i32;
        let cursor_col = display_cursor.column() as i32;
        let cursor_screen_row = cursor_row - display_snapshot.scroll_y as i32;
        let cursor_screen_col = cursor_col - display_snapshot.scroll_x as i32;

        editor::update_cursor_position(
            stdout,
            editor,
            &display_snapshot,
            editor_inner_rect,
            editor_gutter_width,
            cursor_screen_col,
            cursor_screen_row,
            &mut self.last_cursor_style,
        )?;

        stdout.flush()?;
        Ok(())
    }
}

fn fill_to_eol(count: usize) {
    for _ in 0..count {
        print!(" ");
    }
}

/// Renders the active command or search input prompt at the bottom row.
pub fn render_command_line(
    stdout: &mut Stdout,
    editor: &Editor,
    rect: layout::Rect,
) -> std::io::Result<()> {
    execute!(
        stdout,
        crossterm::style::SetForegroundColor(editor.theme.fg),
        crossterm::style::SetBackgroundColor(editor.theme.gutter),
        MoveTo(rect.x, rect.y)
    )?;
    fill_to_eol(rect.width as usize);
    execute!(stdout, MoveTo(rect.x, rect.y))?;

    let mut cmd_char = ':';
    if editor.search {
        cmd_char = '/';
        if editor.pattern {
            cmd_char = '?';
        }
    }
    print!(
        "{}{}",
        cmd_char,
        editor
            .cmd
            .buffer()
            .row_text(editor.cmd.buffer().row_count() - 1)
    );
    Ok(())
}
