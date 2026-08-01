use super::layout::Rect;
use super::window::Window;

pub struct Popup {
    pub window: Window,
    pub rect: Rect,
}

impl Popup {
    pub fn new(window_id: Option<usize>, x: u16, y: u16, width: u16, height: u16) -> Self {
        // If window_id is not provided, we can use a placeholder like 0, or let the caller/Ui allocate it.
        // But since we want to create it, we can just use the provided ID or default to 0.
        // Actually, we can generate a temporary unique ID or let the caller manage it.
        // Let's check: can we just keep a default or use the provided ID?
        let id = window_id.unwrap_or(0);
        let mut window = Window::new(id, String::new());
        window.draw_border = true;
        window.draw_title = false;
        
        Self {
            window,
            rect: Rect { x, y, width, height },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_popup_creation() {
        let popup = Popup::new(Some(10), 5, 5, 20, 10);
        assert_eq!(popup.window.id, 10);
        assert_eq!(popup.rect.x, 5);
        assert_eq!(popup.rect.y, 5);
        assert_eq!(popup.rect.width, 20);
        assert_eq!(popup.rect.height, 10);
    }
}
