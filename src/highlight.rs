use std::{collections::HashMap, path::Path};

use rope::Point;
use syntect::{
    LoadingError,
    easy::HighlightLines,
    highlighting::{Color, Style, Theme, ThemeSet, ThemeSettings},
    parsing::{SyntaxReference, SyntaxSet},
};
use text::{Buffer, ToOffset};

fn load_theme(tm_file: &str) -> Result<Theme, LoadingError> {
    let tm_path = Path::new(tm_file);
    ThemeSet::get_theme(tm_path)
}

pub struct StyleCache {
    pub styles: Vec<(Style, u32, u32)>,
}

pub struct Highlights {
    theme: Theme,
    syntax_set: SyntaxSet,
    syntax: SyntaxReference,
    style_cache: HashMap<u32, StyleCache>,
    // theme extras
    pub comment: Color,
}

fn row_text(buffer: &Buffer, row: u32) -> String {
    let start = Point::new(row, 0).to_offset(buffer);
    let end = Point::new(row, buffer.line_len(row)).to_offset(buffer);
    buffer.as_rope().chunks_in_range(start..end).collect()
}

impl Highlights {
    pub fn new(file_path: &str, theme_path: &str) -> Self {
        let extension = Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .unwrap_or_default();

        let syntax_set = SyntaxSet::load_defaults_newlines(); // Changed to handle new lines for better syntax parsing
        let theme_set = ThemeSet::load_defaults();

        let default_theme = theme_set.themes.get("base16-ocean.dark").unwrap();
        let theme = load_theme(theme_path).unwrap_or_else(|_| {
            theme_set
                .themes
                .get(theme_path)
                .cloned()
                .unwrap_or_else(|| default_theme.clone())
        });

        let syntax = syntax_set
            .find_syntax_by_extension(&extension)
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

        let mut hl = HighlightLines::new(&syntax, &theme);
        let ranges = hl.highlight_line(" ", &syntax_set).unwrap();
        let default_style = ranges.first().map(|(style, _)| style.clone()).unwrap();
        let mut comment = default_style.background;

        for item in &theme.scopes {
            for selector in &item.scope.selectors {
                if let Some(scope) = selector.extract_single_scope() {
                    if scope.to_string().contains("comment") {
                        let style = item.style;
                        if let Some(fg) = style.foreground {
                            comment = fg;
                        }
                        if let Some(fg) = style.background {
                            comment = fg;
                        }
                    }
                }
            }
        }

        Self {
            theme: theme.clone(),
            syntax_set: syntax_set.clone(),
            syntax: syntax.clone(),
            style_cache: HashMap::new(),
            comment: comment,
        }
    }

    pub fn highlight_lines(&mut self, buffer: &Buffer, start_row: u32, row_count: u32) {
        self.style_cache.clear();

        if row_count == 0 || start_row >= buffer.row_count() {
            return;
        }

        // Syntect parsing is stateful across lines. Parse from the beginning so
        // multiline strings/comments are correct, but only retain requested rows.
        let end_row = start_row.saturating_add(row_count).min(buffer.row_count());
        let mut highlighter = HighlightLines::new(&self.syntax, &self.theme);

        for row in 0..end_row {
            let text = row_text(buffer, row) + "\n";
            let ranges = highlighter
                .highlight_line(&text, &self.syntax_set)
                .expect("syntax highlighting failed");

            if row < start_row {
                continue;
            }

            let mut styles = Vec::with_capacity(ranges.len());
            let mut column = 0_u32;
            for (style, text) in ranges {
                let start_column = column;
                column += text.len() as u32;
                styles.push((style, start_column, column));
            }

            self.style_cache.insert(row, StyleCache { styles });
        }
    }

    pub fn name(&self) -> String {
        self.syntax.name.clone()
    }

    pub fn render_row(&self, row: u32) -> Option<&StyleCache> {
        self.style_cache.get(&row)
    }

    pub fn contains_rows(&self, start_row: u32, end_row: u32) -> bool {
        (start_row..end_row).all(|row| self.style_cache.contains_key(&row))
    }

    pub fn theme_settings(&self) -> &ThemeSettings {
        return &self.theme.settings;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clock::ReplicaId;
    use text::BufferId;

    fn buffer(text: &str) -> Buffer {
        Buffer::new(ReplicaId::LOCAL, BufferId::new(1).unwrap(), text.to_owned())
    }

    #[test]
    fn highlights_requested_rows_with_multiline_context() {
        let buffer = buffer("fn main() {\n/* comment\nstill comment */\nlet value = 1;\n}");
        let mut highlights = Highlights::new("test.rs", "base16-ocean.dark");

        highlights.highlight_lines(&buffer, 2, 2);

        assert!(highlights.render_row(0).is_none());
        assert!(highlights.render_row(1).is_none());
        assert!(highlights.render_row(2).is_some());
        assert!(highlights.render_row(3).is_some());
        assert!(highlights.contains_rows(2, 4));
        assert!(!highlights.contains_rows(1, 4));
    }

    #[test]
    fn style_ranges_use_buffer_byte_columns() {
        let buffer = buffer("let café = 1;");
        let mut highlights = Highlights::new("test.rs", "base16-ocean.dark");

        highlights.highlight_lines(&buffer, 0, 1);

        let styles = &highlights.render_row(0).unwrap().styles;
        assert_eq!(styles.first().unwrap().1, 0);
        assert_eq!(styles.last().unwrap().2, buffer.line_len(0) + 1);
    }

    #[test]
    fn replaces_cache_when_viewport_changes() {
        let buffer = buffer("one\ntwo\nthree\nfour");
        let mut highlights = Highlights::new("test.rs", "base16-ocean.dark");

        highlights.highlight_lines(&buffer, 0, 2);
        assert!(highlights.contains_rows(0, 2));

        highlights.highlight_lines(&buffer, 2, 2);
        assert!(!highlights.contains_rows(0, 2));
        assert!(highlights.contains_rows(2, 4));
    }
}
