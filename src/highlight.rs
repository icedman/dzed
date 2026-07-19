use std::{collections::HashMap, path::Path};

use rope::Point;
use syntect::{
    LoadingError,
    easy::HighlightLines,
    highlighting::{Color, HighlightState, Style, Theme, ThemeSet, ThemeSettings},
    parsing::{ParseState, SyntaxReference, SyntaxSet},
};
use text::{Buffer, ToOffset};

const START_OFFSET: usize = 240;
const CACHE_INTERVAL: usize = 80;

fn load_theme(tm_file: &str) -> Result<Theme, LoadingError> {
    let tm_path = Path::new(tm_file);
    ThemeSet::get_theme(tm_path)
}

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
    pub line_number: usize,
    pub highlight_state: HighlightState,
    pub parser_state: ParseState,
}

pub struct StyleCache {
    pub line_number: usize,
    pub styles: Vec<(Style, u32, u32)>, // Corrected typo for list of tuples
}

pub struct Highlights {
    theme: Theme,
    syntax_set: SyntaxSet,
    syntax: SyntaxReference,
    state_cache: HashMap<usize, StateCache>,
    style_cache: HashMap<usize, StyleCache>,
    highlight_start: usize,
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
            state_cache: HashMap::<usize, StateCache>::new(),
            style_cache: HashMap::<usize, StyleCache>::new(),
            comment: comment,
            highlight_start: 0,
        }
    }

    pub fn update_from_line(&mut self, threshold: usize) {
        self.state_cache
            .retain(|&k, _| k <= threshold.saturating_sub(4));
    }

    pub fn highlight_lines(&mut self, buffer: &Buffer, start: usize, count: usize) {
        self.style_cache.clear();
        let mut hl = HighlightLines::new(&self.syntax, &self.theme);

        // todo START_OFFSET should consider visible rows
        let mut sub_start: usize = start.saturating_sub(START_OFFSET);

        self.highlight_start = 0;

        if let Some((_key, value)) =
            find_entry::<StateCache>(&self.state_cache, start.saturating_sub(CACHE_INTERVAL))
        {
            let ln = value.line_number;
            if ln > sub_start && ln < start {
                sub_start = ln;
                self.highlight_start = ln;
                hl = HighlightLines::from_state(
                    &self.theme,
                    value.highlight_state.clone(),
                    value.parser_state.clone(),
                );
            }
        }

        let end = start + count;
        for row in sub_start..end {
            let text = row_text(buffer, row as u32) + "\n";
            let ranges = hl
                .highlight_line(&text, &self.syntax_set)
                .expect("handle empty range");
            let mut vec = Vec::<(Style, u32, u32)>::new();
            let mut col = 0;
            for (style, text) in ranges.iter() {
                let start = col;
                let end = start + text.chars().count();
                col = end;
                vec.push((style.clone(), start as u32, end as u32));
            }
            self.style_cache.insert(
                row,
                StyleCache {
                    line_number: row,
                    styles: vec,
                },
            );

            // state cache
            if row % CACHE_INTERVAL == 0 {
                let (hs, ps) = hl.state();
                self.state_cache.insert(
                    row,
                    StateCache {
                        line_number: row,
                        highlight_state: hs.clone(),
                        parser_state: ps.clone(),
                    },
                );
                hl = HighlightLines::from_state(&self.theme, hs, ps);
            }
        }
    }

    pub fn name(&self) -> String {
        self.syntax.name.clone()
    }

    pub fn render_line(&self, line: usize) -> Option<&StyleCache> {
        self.style_cache.get(&line)
    }

    // Prepare default background pair
    pub fn get_default_style(&self) -> Style {
        let mut hl = HighlightLines::new(&self.syntax, &self.theme);
        let ranges = hl.highlight_line(" ", &self.syntax_set).unwrap();
        ranges.first().map(|(style, _)| style.clone()).unwrap()
    }

    pub fn theme_settings(&self) -> &ThemeSettings {
        return &self.theme.settings;
    }

    pub fn stats(&self) -> (usize, usize) {
        let cache_len = self.state_cache.len();
        let highlight_start = self.highlight_start;
        (cache_len, highlight_start)
    }
}
