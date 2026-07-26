use crossterm;
use std::path::Path;
use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet};

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub r#type: Option<String>, // "light" or "dark"
}

#[derive(Debug, Clone, Deserialize)]
pub struct ColorSchemeFile {
    pub metadata: Metadata,
    pub colors: HashMap<String, String>,
    pub ui: HashMap<String, String>,
    pub syntax: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub metadata: Metadata,
    pub colors: HashMap<String, crossterm::style::Color>,
    pub ui: HashMap<String, crossterm::style::Color>,
    pub syntax: HashMap<String, crossterm::style::Color>,
}

impl ColorScheme {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let parsed: ColorSchemeFile = toml::from_str(&contents)?;

        let mut colors = HashMap::new();
        for (k, v) in &parsed.colors {
            if let Some(color) = parse_hex_color(v) {
                colors.insert(k.clone(), color);
            }
        }

        let mut ui = HashMap::new();
        for (k, v) in &parsed.ui {
            if let Some(color) = resolve_color(v, &parsed.colors, &parsed.ui) {
                ui.insert(k.clone(), color);
            }
        }

        let mut syntax = HashMap::new();
        for (k, v) in &parsed.syntax {
            if let Some(color) = resolve_color(v, &parsed.colors, &parsed.syntax) {
                syntax.insert(k.clone(), color);
            }
        }

        Ok(Self {
            metadata: parsed.metadata,
            colors,
            ui,
            syntax,
        })
    }
}

fn parse_hex_color(hex: &str) -> Option<crossterm::style::Color> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(crossterm::style::Color::Rgb { r, g, b })
    } else if hex.len() == 3 {
        let r_char = &hex[0..1];
        let g_char = &hex[1..2];
        let b_char = &hex[2..3];
        let r = u8::from_str_radix(&format!("{}{}", r_char, r_char), 16).ok()?;
        let g = u8::from_str_radix(&format!("{}{}", g_char, g_char), 16).ok()?;
        let b = u8::from_str_radix(&format!("{}{}", b_char, b_char), 16).ok()?;
        Some(crossterm::style::Color::Rgb { r, g, b })
    } else {
        None
    }
}

fn resolve_color(
    val: &str,
    palette: &HashMap<String, String>,
    fallback_map: &HashMap<String, String>,
) -> Option<crossterm::style::Color> {
    resolve_color_recursive(val, palette, fallback_map, 0)
}

fn resolve_color_recursive(
    val: &str,
    palette: &HashMap<String, String>,
    fallback_map: &HashMap<String, String>,
    depth: usize,
) -> Option<crossterm::style::Color> {
    if depth > 10 {
        return None;
    }
    if let Some(resolved) = palette.get(val) {
        parse_hex_color(resolved)
    } else if let Some(linked_val) = fallback_map.get(val) {
        resolve_color_recursive(linked_val, palette, fallback_map, depth + 1)
    } else {
        parse_hex_color(val)
    }
}

pub trait ToCrossTerm {
    fn rgb(&self) -> crossterm::style::Color;
}

pub trait ColorAdjust {
    fn lighten(&self, amount: u8) -> syntect::highlighting::Color;
    fn darken(&self, amount: u8) -> syntect::highlighting::Color;
}

impl ToCrossTerm for syntect::highlighting::Color {
    fn rgb(&self) -> crossterm::style::Color {
        return crossterm::style::Color::Rgb {
            r: self.r,
            g: self.g,
            b: self.b,
        };
    }
}

impl ColorAdjust for syntect::highlighting::Color {
    fn lighten(&self, amount: u8) -> syntect::highlighting::Color {
        syntect::highlighting::Color {
            r: self.r.saturating_add(amount),
            g: self.g.saturating_add(amount),
            b: self.b.saturating_add(amount),
            a: self.a,
        }
    }

    fn darken(&self, amount: u8) -> syntect::highlighting::Color {
        syntect::highlighting::Color {
            r: self.r.saturating_sub(amount),
            g: self.g.saturating_sub(amount),
            b: self.b.saturating_sub(amount),
            a: self.a,
        }
    }
}

fn load_theme(theme_name_or_path: &str) -> syntect::highlighting::Theme {
    let theme_set = ThemeSet::load_defaults();
    let default_theme = theme_set.themes.get("base16-ocean.dark").unwrap();
    let tm_path = Path::new(theme_name_or_path);
    let theme = ThemeSet::get_theme(tm_path).unwrap_or_else(|_| {
        theme_set
            .themes
            .get(theme_name_or_path)
            .cloned()
            .unwrap_or_else(|| default_theme.clone())
    });

    theme
}

fn crossterm_color(r: u8, g: u8, b: u8) -> crossterm::style::Color {
    crossterm::style::Color::Rgb { r, g, b }
}

pub struct Theme {
    pub theme: syntect::highlighting::Theme,
    pub fg: crossterm::style::Color,
    pub bg: crossterm::style::Color,
    pub caret: crossterm::style::Color,
    pub select: crossterm::style::Color,
    pub comment: crossterm::style::Color,
    pub keyword: crossterm::style::Color,
    pub string: crossterm::style::Color,
    pub constant: crossterm::style::Color,
    pub number: crossterm::style::Color,
    pub find: crossterm::style::Color,
    pub find_fg: crossterm::style::Color,
    pub gutter: crossterm::style::Color,
    pub gutter_fg: crossterm::style::Color,
}

impl Theme {
    pub fn new(theme_path: &str) -> Self {
        let mut me = Self {
            theme: load_theme(theme_path),
            fg: crossterm_color(200, 200, 200),
            bg: crossterm_color(50, 50, 50),
            caret: crossterm_color(200, 200, 200),
            select: crossterm_color(100, 100, 100),
            comment: crossterm_color(80, 80, 80),
            keyword: crossterm_color(80, 80, 80),
            string: crossterm_color(80, 80, 80),
            constant: crossterm_color(80, 80, 80),
            number: crossterm_color(80, 80, 80),
            find_fg: crossterm_color(200, 200, 200),
            find: crossterm_color(50, 50, 50),
            gutter_fg: crossterm_color(200, 200, 200),
            gutter: crossterm_color(50, 50, 50),
        };
        me.extract_colors();
        return me;
    }

    fn extract_colors(&mut self) {
        let settings = &self.theme.settings;
        let fg = settings.foreground.unwrap();
        let bg = settings.background.unwrap();

        self.fg = fg.rgb();
        self.bg = bg.darken(10).rgb();
        let raw_caret = settings.caret.unwrap_or(fg);
        let mixed_caret = syntect::highlighting::Color {
            r: ((raw_caret.r as f32 * 0.4) + (bg.r as f32 * 0.6)) as u8,
            g: ((raw_caret.g as f32 * 0.4) + (bg.g as f32 * 0.6)) as u8,
            b: ((raw_caret.b as f32 * 0.4) + (bg.b as f32 * 0.6)) as u8,
            a: raw_caret.a,
        };
        self.caret = mixed_caret.rgb();
        self.select = settings.selection.unwrap_or(bg.darken(10)).rgb();
        self.find_fg = settings.find_highlight_foreground.unwrap_or(fg).rgb();
        self.find = settings.find_highlight.unwrap_or(bg.darken(10)).rgb();
        self.gutter = bg.darken(10).rgb();
        self.gutter_fg = self.comment;

        // run a highlighter to extract some colors
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax = syntax_set
            .find_syntax_by_extension("c")
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

        let mut hl = HighlightLines::new(&syntax, &self.theme);
        let ranges = hl.highlight_line(" ", &syntax_set).unwrap();
        let default_style = ranges.first().map(|(style, _)| style.clone()).unwrap();
        self.comment = default_style.background.rgb();

        for item in &self.theme.scopes {
            for selector in &item.scope.selectors {
                if let Some(scope) = selector.extract_single_scope() {
                    let style = item.style;
                    if scope.to_string().contains("comment") {
                        if let Some(clr) = style.foreground {
                            self.comment = clr.rgb();
                        } else if let Some(clr) = style.background {
                            self.comment = clr.rgb();
                        }
                    }
                    if scope.to_string().contains("keyword") {
                        if let Some(clr) = style.foreground {
                            self.keyword = clr.rgb();
                        }
                    }
                    if scope.to_string().contains("constant") {
                        if let Some(clr) = style.foreground {
                            self.constant = clr.rgb();
                        }
                    }
                    if scope.to_string().contains("string") {
                        if let Some(clr) = style.foreground {
                            self.string = clr.rgb();
                        }
                    }
                    if scope.to_string().contains("number") {
                        if let Some(clr) = style.foreground {
                            self.number = clr.rgb();
                            self.gutter_fg = self.number;
                        }
                    }
                }
            }
        }
    }

    pub fn load_theme(&mut self, theme_path: &str) {
        self.theme = load_theme(theme_path);
        self.extract_colors();
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colorscheme_parsing() {
        let toml_content = r##"
            [metadata]
            name = "catppuccin-mocha"
            description = "Soothing pastel theme for the high-spirited!"
            author = "Catppuccin Community"
            type = "dark"

            [colors]
            base = "#1e1e2e"
            text = "#cdd6f4"
            rosewater = "#f5e0dc"
            mauve = "#cba6f7"
            sky = "#89dceb"

            [ui]
            foreground = "text"
            background = "base"
            caret = "rosewater"
            selection = "foreground"

            [syntax]
            comment = "#6c7086"
            keyword = "mauve"
            operator = "sky"
            function = "keyword"
        "##;

        let path = "temp_colorscheme_test.toml";
        std::fs::write(path, toml_content).unwrap();

        let scheme = ColorScheme::load_from_file(path).unwrap();
        
        assert_eq!(scheme.metadata.name, "catppuccin-mocha");
        assert_eq!(scheme.metadata.r#type.as_deref(), Some("dark"));

        // Verify resolved color values
        let fg_color = scheme.ui.get("foreground").unwrap();
        let bg_color = scheme.ui.get("background").unwrap();
        let selection_color = scheme.ui.get("selection").unwrap();
        let comment_color = scheme.syntax.get("comment").unwrap();
        let function_color = scheme.syntax.get("function").unwrap();

        // Verify resolved palette colors map
        let base_palette = scheme.colors.get("base").unwrap();
        assert_eq!(
            base_palette,
            &crossterm::style::Color::Rgb { r: 30, g: 30, b: 46 }
        );

        assert_eq!(
            bg_color,
            &crossterm::style::Color::Rgb { r: 30, g: 30, b: 46 }
        );
        assert_eq!(
            fg_color,
            &crossterm::style::Color::Rgb { r: 205, g: 214, b: 244 }
        );
        assert_eq!(
            selection_color,
            &crossterm::style::Color::Rgb { r: 205, g: 214, b: 244 }
        );
        assert_eq!(
            comment_color,
            &crossterm::style::Color::Rgb { r: 108, g: 112, b: 134 }
        );
        assert_eq!(
            function_color,
            &crossterm::style::Color::Rgb { r: 203, g: 166, b: 247 }
        );

        std::fs::remove_file(path).unwrap();
    }
}
