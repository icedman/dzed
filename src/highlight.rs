use std::{collections::HashMap, path::Path};

use rope::Point;
use syntect::{
    LoadingError,
    easy::HighlightLines,
    highlighting::{HighlightState, Style, Theme, ThemeSet},
    parsing::{ParseState, SyntaxReference, SyntaxSet},
};
use text::{Buffer, ToOffset};

const START_OFFSET: u32 = 240;
const CACHE_INTERVAL: u32 = 80;

fn find_entry<T>(state_cache: &HashMap<usize, T>, target: usize) -> Option<(&usize, &T)> {
    let mut nearest_key = None;
    let mut min_diff = usize::MAX;

    for key in state_cache.keys() {
        if *key == target {
            return Some((key, state_cache.get(key).unwrap()));
        } else if *key > target && (*key - target) < min_diff {
            nearest_key = Some(key);
            min_diff = *key - target;
        }
    }

    nearest_key.map(|key| (key, state_cache.get(key).unwrap()))
}

pub struct StateCache {
    pub line_number: u32,
    pub highlight_state: HighlightState,
    pub parser_state: ParseState,
}

pub struct StyleCache {
    pub styles: Vec<(Style, u32, u32)>,
}

pub struct Highlights {
    syntax_set: SyntaxSet,
    syntax: SyntaxReference,
    state_cache: HashMap<usize, StateCache>,
    style_cache: HashMap<u32, StyleCache>,
    highlight_start: u32,
}

fn row_text(buffer: &Buffer, row: u32) -> String {
    let start = Point::new(row, 0).to_offset(buffer);
    let end = Point::new(row, buffer.line_len(row)).to_offset(buffer);
    buffer.as_rope().chunks_in_range(start..end).collect()
}

impl Highlights {
    pub fn new(file_path: &str) -> Self {
        let extension = Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .unwrap_or_default();

        let syntax_set = SyntaxSet::load_defaults_newlines(); // Changed to handle new lines for better syntax parsing

        let syntax = syntax_set
            .find_syntax_by_extension(&extension)
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

        Self {
            syntax_set: syntax_set.clone(),
            syntax: syntax.clone(),
            state_cache: HashMap::new(),
            style_cache: HashMap::new(),
            highlight_start: 0,
        }
    }

    pub fn highlight_lines(
        &mut self,
        buffer: &Buffer,
        start_row: u32,
        row_count: u32,
        theme: &Theme,
    ) {
        self.style_cache.clear();
        let mut cached_highlighter: Option<HighlightLines> = None;

        if row_count == 0 || start_row >= buffer.row_count() {
            return;
        }

        let mut start: u32 = start_row.saturating_sub(START_OFFSET);

        if let Some((_key, value)) = find_entry::<StateCache>(
            &self.state_cache,
            start_row.saturating_sub(CACHE_INTERVAL) as usize,
        ) {
            let ln: u32 = value.line_number as u32;
            if ln > start && ln < start_row {
                start = ln;
                self.highlight_start = ln;
                cached_highlighter = Some(HighlightLines::from_state(
                    theme,
                    value.highlight_state.clone(),
                    value.parser_state.clone(),
                ));
            }
        }

        let mut highlighter = match cached_highlighter {
            Some(chl) => chl,
            None => HighlightLines::new(&self.syntax, theme),
        };

        // Syntect parsing is stateful across lines. Parse from the beginning so
        // multiline strings/comments are correct, but only retain requested rows.
        let end_row = std::cmp::min(buffer.row_count(), start_row.saturating_add(row_count));

        for row in start..end_row {
            let text = row_text(buffer, row) + "\n";
            let ranges = highlighter
                .highlight_line(&text, &self.syntax_set)
                .expect("syntax highlighting failed");

            let mut styles = Vec::with_capacity(ranges.len());
            let mut column = 0_u32;
            for (style, text) in ranges {
                let start_column = column;
                column += text.len() as u32;
                styles.push((style, start_column, column));
            }

            self.style_cache.insert(row, StyleCache { styles });

            // state cache
            if row % CACHE_INTERVAL == 0 {
                let (hs, ps) = highlighter.state();
                self.state_cache.insert(
                    row as usize,
                    StateCache {
                        line_number: row,
                        highlight_state: hs.clone(),
                        parser_state: ps.clone(),
                    },
                );
                highlighter = HighlightLines::from_state(theme, hs, ps);
            }
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

    pub fn invalidate_state(&mut self, start_row: u32) {
        self.state_cache.retain(|&row, _| row < start_row as usize);
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
