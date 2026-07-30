use super::layout::Rect;
use super::views::View;
use crate::controller::controllers::ViewController;
use crate::{editor::Editor};

use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};
use std::io::Write;

pub struct Window {
    pub id: usize,
    pub title: String,
    pub draw_border: bool,
    pub view: Option<Box<dyn View>>,
    pub controller: Option<Box<dyn ViewController>>,
    pub buffer_id: Option<usize>,
}

impl Window {
    pub fn new(id: usize, title: String) -> Self {
        Self {
            id,
            title,
            draw_border: true,
            view: None,
            controller: None,
            buffer_id: None,
        }
    }

    pub fn set_view(&mut self, view: Box<dyn View>) {
        self.view = Some(view);
    }

    pub fn set_controller(&mut self, controller: Box<dyn ViewController>) {
        self.controller = Some(controller);
    }

    pub fn draw<W: Write>(
        &mut self,
        w: &mut W,
        rect: Rect,
        editor: &Editor,
    ) -> std::io::Result<()> {
        if rect.width == 0 || rect.height == 0 {
            return Ok(());
        }

        if self.draw_border {
            let border_fg = Color::Cyan;

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
            _ = view.draw(w, inner_rect, editor);
        }

        Ok(())
    }
}
