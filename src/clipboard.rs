#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ClipboardKind {
    #[default]
    Character,
    Line,
    Block,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Clipboard {
    text: String,
    kind: ClipboardKind,
}

impl Clipboard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, text: impl Into<String>, kind: ClipboardKind) {
        self.text = text.into();
        self.kind = kind;
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.set(text, ClipboardKind::Character);
    }

    pub fn set_lines(&mut self, text: impl Into<String>) {
        self.set(text, ClipboardKind::Line);
    }

    pub fn set_block(&mut self, text: impl Into<String>) {
        self.set(text, ClipboardKind::Block);
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn kind(&self) -> ClipboardKind {
        self.kind
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.kind = ClipboardKind::Character;
    }

    pub fn take(&mut self) -> (String, ClipboardKind) {
        let text = std::mem::take(&mut self.text);
        let kind = std::mem::take(&mut self.kind);
        (text, kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_text_and_selection_kind() {
        let mut clipboard = Clipboard::new();
        clipboard.set_lines("one\ntwo\n");

        assert_eq!(clipboard.text(), "one\ntwo\n");
        assert_eq!(clipboard.kind(), ClipboardKind::Line);
        assert!(!clipboard.is_empty());
    }

    #[test]
    fn taking_contents_resets_clipboard() {
        let mut clipboard = Clipboard::new();
        clipboard.set_block("ab\ncd");

        assert_eq!(
            clipboard.take(),
            ("ab\ncd".to_string(), ClipboardKind::Block)
        );
        assert!(clipboard.is_empty());
        assert_eq!(clipboard.kind(), ClipboardKind::Character);
    }
}
