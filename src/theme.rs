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
}

impl Theme {
    pub fn new(theme_path: &str) -> Self {
        let mut me = Self {
            theme: load_theme(theme_path),
            fg: crossterm_color(200, 200, 200),
            bg: crossterm_color(50, 50, 50),
            caret: crossterm_color(200, 200, 200),
            select: crossterm_color(100, 100, 100),
        };
        me.extract_colors();
        return me;
    }

    pub fn load_theme(&mut self, theme_path: &str) {
        self.theme = load_theme(theme_path);
        self.extract_colors();
    }

    fn extract_colors(&mut self) {
        let settings = &self.theme.settings;
        let fg = settings.foreground.unwrap_or(syntect::highlighting::Color::WHITE);
        let bg = settings.background.unwrap_or(syntect::highlighting::Color::BLACK);
        self.fg = fg.rgb();
        self.bg = bg.rgb();
        self.caret = settings.caret.unwrap_or(fg).rgb();
        self.select = settings.selection.unwrap_or(bg).rgb();
    }
}



