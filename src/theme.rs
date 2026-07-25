use crossterm;
use std::path::Path;
use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet};

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
