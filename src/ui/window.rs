use super::layout::Rect;
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};
use std::io::Write;

pub struct Window {
    pub id: usize,
    pub title: String,
    pub is_focused: bool,
    pub draw_border: bool,
    pub view_id: Option<usize>,
}

impl Window {
    pub fn new(id: usize, title: String) -> Self {
        Self {
            id,
            title,
            is_focused: false,
            draw_border: true,
            view_id: None,
        }
    }

    pub fn draw<W: Write>(&mut self, w: &mut W, rect: Rect) -> std::io::Result<()> {
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

        Ok(())
    }
}
