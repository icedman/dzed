use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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
