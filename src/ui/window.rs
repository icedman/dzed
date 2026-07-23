use super::layout::Rect;
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};
use std::io::Write;

use super::view::View;

pub struct Window {
    pub id: usize,
    pub title: String,
    pub is_focused: bool,
    pub view: Option<Box<dyn View>>,
    pub draw_border: bool,
}

impl Window {
    pub fn new(id: usize, title: String) -> Self {
        Self {
            id,
            title,
            is_focused: false,
            view: None,
            draw_border: true,
        }
    }

    pub fn set_view(&mut self, view: Box<dyn View>) {
        self.view = Some(view);
    }

    /// Renders the window (border if configured) along with its inner view.
    pub fn draw<W: Write>(
        &mut self,
        w: &mut W,
        rect: Rect,
        editor: &mut crate::editor::Editor,
        last_cursor_style: &mut Option<crossterm::cursor::SetCursorStyle>,
    ) -> std::io::Result<()> {
        if rect.width == 0 || rect.height == 0 {
            return Ok(());
        }

        if self.draw_border {
            let border_fg = if self.is_focused {
                Color::Yellow
            } else {
                Color::DarkGrey
            };

            // Draw border
            execute!(w, SetForegroundColor(border_fg))?;

            // Draw top border
            execute!(w, MoveTo(rect.x, rect.y))?;
            if rect.width > 2 {
                let title_len = self.title.chars().count();
                if title_len + 4 < rect.width as usize {
                    let left_len = (rect.width as usize - title_len - 4) / 2;
                    let right_len = rect.width as usize - title_len - 4 - left_len;
                    execute!(
                        w,
                        Print(format!(
                            "┌{} {} {}┐",
                            "─".repeat(left_len),
                            self.title,
                            "─".repeat(right_len)
                        ))
                    )?;
                } else {
                    execute!(
                        w,
                        Print(format!("┌{}┐", "─".repeat(rect.width as usize - 2)))
                    )?;
                }
            } else {
                execute!(
                    w,
                    Print("┌┐".chars().take(rect.width as usize).collect::<String>())
                )?;
            }

            // Draw sides
            for y in 1..rect.height.saturating_sub(1) {
                execute!(w, MoveTo(rect.x, rect.y + y))?;
                if rect.width > 1 {
                    execute!(w, Print("│"))?;
                    execute!(w, MoveTo(rect.x + rect.width - 1, rect.y + y))?;
                    execute!(w, Print("│"))?;
                } else {
                    execute!(w, Print("│"))?;
                }
            }

            // Draw bottom border
            if rect.height > 1 {
                execute!(w, MoveTo(rect.x, rect.y + rect.height - 1))?;
                if rect.width > 1 {
                    execute!(
                        w,
                        Print(format!("└{}┘", "─".repeat(rect.width as usize - 2)))
                    )?;
                } else {
                    execute!(w, Print("└"))?;
                }
            }

            execute!(w, ResetColor)?;
        }

        // Draw inner view content
        if let Some(ref mut view) = self.view {
            let inner_rect = if self.draw_border {
                Rect {
                    x: rect.x.saturating_add(1),
                    y: rect.y.saturating_add(1),
                    width: rect.width.saturating_sub(2),
                    height: rect.height.saturating_sub(2),
                }
            } else {
                rect
            };
            view.draw(w, inner_rect, editor, last_cursor_style)?;
        }

        Ok(())
    }
}
