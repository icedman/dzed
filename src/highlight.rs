use std::{collections::HashMap, path::Path};

use clock::Global;
use rope::Point;
use syntect::{
    easy::ScopeRangeIterator,
    highlighting::Style,
    parsing::{ParseState, SyntaxReference, SyntaxSet, ScopeStack},
};
use text::{BufferSnapshot, ToOffset};

const ENABLE_STATE_CACHE: bool = true;
const CACHE_INTERVAL: u32 = 32;
const START_OFFSET: u32 = 1024;

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

#[derive(Clone)]
pub struct StateCache {
    pub line_number: u32,
    pub parser_state: ParseState,
    pub highlight_state: Option<syntect::highlighting::HighlightState>,
    pub scope_stack: Option<ScopeStack>,
}

#[derive(Clone)]
pub struct StyleCache {
    pub styles: Vec<(Style, u32, u32)>,
}

pub struct Highlights {
    syntax_set: SyntaxSet,
    syntax: SyntaxReference,
    state_cache: HashMap<usize, StateCache>,
    style_cache: HashMap<u32, StyleCache>,
    highlight_start: u32,
    pub last_snapshot_version: Option<Global>,
}

fn row_text(buffer: &BufferSnapshot, row: u32) -> String {
    let start = Point::new(row, 0).to_offset(buffer);
    let end = Point::new(row, buffer.line_len(row)).to_offset(buffer);
    buffer.as_rope().chunks_in_range(start..end).collect()
}

fn map_scope_to_style(scopes: &[syntect::parsing::Scope], colorscheme: &crate::colorscheme::ColorScheme) -> Style {
    let mut resolved_style = crate::colorscheme::Style {
        color: colorscheme.ui.get("foreground").map(|s| s.color).unwrap_or(crossterm::style::Color::White),
        bold: false,
        italic: false,
        underline: false,
        strikethrough: false,
    };

    for scope in scopes.iter().rev() {
        let scope_str = scope.to_string();
        
        let key = if scope_str.contains("comment") {
            Some("comment")
        } else if scope_str.contains("string") {
            Some("string")
        } else if scope_str.contains("constant") || scope_str.contains("numeric") || scope_str.contains("boolean") {
            Some("constant")
        } else if scope_str.contains("keyword") || scope_str.contains("storage") {
            Some("keyword")
        } else if scope_str.contains("entity.name.function") || scope_str.contains("support.function") || scope_str.contains("variable.function") {
            Some("function")
        } else if scope_str.contains("variable") || scope_str.contains("parameter") {
            Some("variable")
        } else if scope_str.contains("keyword.operator") || scope_str.contains("punctuation.section") || scope_str.contains("punctuation.separator") {
            Some("operator")
        } else if scope_str.contains("entity.name.type") || scope_str.contains("support.type") || scope_str.contains("storage.type") {
            Some("type")
        } else {
            None
        };

        if let Some(k) = key {
            if let Some(style) = colorscheme.syntax.get(k) {
                resolved_style = style.clone();
                break;
            }
        }
    }

    let mut font_style = syntect::highlighting::FontStyle::empty();
    if resolved_style.bold { font_style.insert(syntect::highlighting::FontStyle::BOLD); }
    if resolved_style.italic { font_style.insert(syntect::highlighting::FontStyle::ITALIC); }
    if resolved_style.underline { font_style.insert(syntect::highlighting::FontStyle::UNDERLINE); }

    let r_fg = match resolved_style.color {
        crossterm::style::Color::Rgb { r, g, b } => syntect::highlighting::Color { r, g, b, a: 255 },
        _ => syntect::highlighting::Color::WHITE,
    };
    let r_bg = match colorscheme.ui.get("background").map(|s| s.color).unwrap_or(crossterm::style::Color::Black) {
        crossterm::style::Color::Rgb { r, g, b } => syntect::highlighting::Color { r, g, b, a: 255 },
        _ => syntect::highlighting::Color::BLACK,
    };

    Style {
        foreground: r_fg,
        background: r_bg,
        font_style,
    }
}

impl Highlights {
    pub fn new(file_path: &str) -> Self {
        let extension = Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .unwrap_or_default();

        let syntax_set = SyntaxSet::load_defaults_newlines();

        let syntax = syntax_set
            .find_syntax_by_extension(&extension)
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

        Self {
            syntax_set: syntax_set.clone(),
            syntax: syntax.clone(),
            state_cache: HashMap::new(),
            style_cache: HashMap::new(),
            highlight_start: 0,
            last_snapshot_version: None,
        }
    }

    pub fn is_sync(&self, buffer: &BufferSnapshot) -> bool {
        self.last_snapshot_version.as_ref() == Some(&buffer.version)
    }

    pub fn highlight_lines(
        &mut self,
        buffer: &BufferSnapshot,
        start_row: u32,
        row_count: u32,
        colorscheme: &crate::colorscheme::ColorScheme,
        theme: &syntect::highlighting::Theme,
        use_colorscheme: bool,
    ) {
        self.last_snapshot_version = Some(buffer.version.clone());
        self.style_cache.clear();
        let mut cached_state: Option<StateCache> = None;

        if row_count == 0 || start_row >= buffer.row_count() {
            return;
        }

        let mut start: u32 = start_row.saturating_sub(START_OFFSET);

        if ENABLE_STATE_CACHE {
            if let Some((_key, value)) = find_entry::<StateCache>(
                &self.state_cache,
                start_row.saturating_sub(CACHE_INTERVAL) as usize,
            ) {
                let ln: u32 = value.line_number as u32;
                if ln > start && ln < start_row {
                    start = ln;
                    self.highlight_start = ln;
                    cached_state = Some(value.clone());
                }
            }
        }

        let end_row = std::cmp::min(buffer.row_count(), start_row.saturating_add(row_count));

        if use_colorscheme {
            let mut parser = match cached_state {
                Some(ref state) => state.parser_state.clone(),
                None => ParseState::new(&self.syntax),
            };
            let mut stack = match cached_state {
                Some(ref state) => state.scope_stack.clone().unwrap_or_else(ScopeStack::new),
                None => ScopeStack::new(),
            };

            for row in start..end_row {
                let text = row_text(buffer, row) + "\n";
                let ops = parser
                    .parse_line(&text, &self.syntax_set)
                    .expect("syntax parsing failed");

                let mut styles = Vec::new();
                let mut column = 0_u32;
                for (range, op) in ScopeRangeIterator::new(&ops, &text) {
                    let _ = stack.apply(&op);
                    let start_column = column;
                    let len = range.end - range.start;
                    column += len as u32;
                    let style = map_scope_to_style(stack.as_slice(), colorscheme);
                    styles.push((style, start_column, column));
                }

                if row >= start_row {
                    self.style_cache.insert(row, StyleCache { styles });
                }

                if ENABLE_STATE_CACHE && row % CACHE_INTERVAL == 0 {
                    self.state_cache.insert(
                        row as usize,
                        StateCache {
                            line_number: row,
                            parser_state: parser.clone(),
                            highlight_state: None,
                            scope_stack: Some(stack.clone()),
                        },
                    );
                }
            }
        } else {
            let mut highlighter = match cached_state {
                Some(state) => {
                    if let Some(hs) = state.highlight_state {
                        syntect::easy::HighlightLines::from_state(theme, hs, state.parser_state)
                    } else {
                        syntect::easy::HighlightLines::new(&self.syntax, theme)
                    }
                }
                None => syntect::easy::HighlightLines::new(&self.syntax, theme),
            };

            for row in start..end_row {
                let text = row_text(buffer, row) + "\n";
                let ranges = highlighter
                    .highlight_line(&text, &self.syntax_set)
                    .expect("syntax highlighting failed");

                let mut styles = Vec::new();
                let mut column = 0_u32;
                for (style, text_span) in ranges {
                    let start_column = column;
                    column += text_span.len() as u32;
                    styles.push((style, start_column, column));
                }

                if row >= start_row {
                    self.style_cache.insert(row, StyleCache { styles });
                }

                if ENABLE_STATE_CACHE && row % CACHE_INTERVAL == 0 {
                    let (hs, ps) = highlighter.state();
                    self.state_cache.insert(
                        row as usize,
                        StateCache {
                            line_number: row,
                            parser_state: ps.clone(),
                            highlight_state: Some(hs.clone()),
                            scope_stack: None,
                        },
                    );
                    highlighter = syntect::easy::HighlightLines::from_state(theme, hs, ps);
                }
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
        self.style_cache.retain(|&row, _| row < start_row);
    }

    pub fn get_style_cache(&self) -> &HashMap<u32, StyleCache> {
        &self.style_cache
    }

    pub fn get_state_cache(&self) -> &HashMap<usize, StateCache> {
        &self.state_cache
    }

    pub fn merge_caches(
        &mut self,
        style_cache: HashMap<u32, StyleCache>,
        state_cache: HashMap<usize, StateCache>,
    ) {
        self.style_cache.extend(style_cache);
        self.state_cache.extend(state_cache);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clock::ReplicaId;
    use text::{Buffer, BufferId};

    fn buffer(text: &str) -> Buffer {
        Buffer::new(ReplicaId::LOCAL, BufferId::new(1).unwrap(), text.to_owned())
    }

    #[test]
    fn highlights_requested_rows_with_multiline_context() {
        let buffer = buffer("fn main() {\n/* comment\nstill comment */\nlet value = 1;\n}");
        let mut highlights = Highlights::new("test.rs");
        let colorscheme = crate::colorscheme::ColorScheme::load_default();
        let theme_set = syntect::highlighting::ThemeSet::load_defaults();
        let theme = &theme_set.themes["base16-ocean.dark"];

        highlights.highlight_lines(&buffer.snapshot(), 2, 2, &colorscheme, theme, true);

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
        let mut highlights = Highlights::new("test.rs");
        let colorscheme = crate::colorscheme::ColorScheme::load_default();
        let theme_set = syntect::highlighting::ThemeSet::load_defaults();
        let theme = &theme_set.themes["base16-ocean.dark"];

        highlights.highlight_lines(&buffer.snapshot(), 0, 1, &colorscheme, theme, true);

        let styles = &highlights.render_row(0).unwrap().styles;
        assert_eq!(styles.first().unwrap().1, 0);
        assert_eq!(styles.last().unwrap().2, buffer.line_len(0) + 1);
    }

    #[test]
    fn replaces_cache_when_viewport_changes() {
        let buffer = buffer("one\ntwo\nthree\nfour");
        let mut highlights = Highlights::new("test.rs");
        let colorscheme = crate::colorscheme::ColorScheme::load_default();
        let theme_set = syntect::highlighting::ThemeSet::load_defaults();
        let theme = &theme_set.themes["base16-ocean.dark"];

        highlights.highlight_lines(&buffer.snapshot(), 0, 2, &colorscheme, theme, true);
        assert!(highlights.contains_rows(0, 2));

        highlights.highlight_lines(&buffer.snapshot(), 2, 2, &colorscheme, theme, true);
        assert!(!highlights.contains_rows(0, 2));
        assert!(highlights.contains_rows(2, 4));
    }
}
