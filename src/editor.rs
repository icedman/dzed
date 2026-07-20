use crate::actions::Mode;
use crate::display::display_map::DisplayMap;
use crate::document::Document;
use crate::highlight::Highlights;

pub struct EditorBuffer {
    pub file_path: String,
    pub doc: Document,
    pub display_map: DisplayMap,
    pub hl: Highlights,
    pub dirty_hl: bool,
}

pub struct EditorTheme {
    pub fg: crossterm::style::Color,
    pub bg: crossterm::style::Color,
    pub caret: crossterm::style::Color,
    pub select: crossterm::style::Color,
    pub find_highlight_fg: crossterm::style::Color,
    pub find_highlight_bg: crossterm::style::Color,
    pub gutter_fg: crossterm::style::Color,
    pub gutter_bg: crossterm::style::Color,
}

impl EditorTheme {
    pub fn from_highlights(hl: &Highlights) -> Self {
        let settings = hl.theme_settings();
        let fg = settings.foreground.unwrap();
        let bg = settings.background.unwrap();
        Self {
            fg: fg.rgb(),
            bg: bg.darken(10).rgb(),
            caret: settings.caret.unwrap_or(fg).rgb(),
            find_highlight_fg: settings
                .find_highlight_foreground
                .unwrap_or(bg.darken(10))
                .rgb(),
            find_highlight_bg: settings.find_highlight.unwrap_or(bg.darken(10)).rgb(),
            select: settings.selection.unwrap_or(bg.darken(10)).rgb(),
            gutter_bg: bg.darken(10).rgb(),
            gutter_fg: hl.comment.rgb(),
        }
    }
}

impl EditorBuffer {
    pub fn new(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let doc = Document::new(file_path)?;
        let hl = Highlights::new(file_path, "./test/themes/Dracula.tmTheme");
        let display_map = DisplayMap::new(doc.buffer().snapshot().clone(), None);
        Ok(Self {
            file_path: file_path.to_string(),
            doc,
            display_map,
            hl,
            dirty_hl: true,
        })
    }
}

pub struct BufferManager {
    pub buffers: Vec<EditorBuffer>,
    pub active_idx: usize,
}

impl BufferManager {
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
            active_idx: 0,
        }
    }

    pub fn add_buffer(&mut self, buffer: EditorBuffer) {
        self.buffers.push(buffer);
        self.active_idx = self.buffers.len() - 1;
    }

    pub fn active(&self) -> &EditorBuffer {
        &self.buffers[self.active_idx]
    }

    pub fn active_mut(&mut self) -> &mut EditorBuffer {
        &mut self.buffers[self.active_idx]
    }

    pub fn switch_next(&mut self) {
        if !self.buffers.is_empty() {
            self.active_idx = (self.active_idx + 1) % self.buffers.len();
        }
    }

    pub fn switch_prev(&mut self) {
        if !self.buffers.is_empty() {
            if self.active_idx == 0 {
                self.active_idx = self.buffers.len() - 1;
            } else {
                self.active_idx -= 1;
            }
        }
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

pub struct Editor {
    pub buffer_manager: BufferManager,
    pub cmd: Document,
    pub command_history: Vec<String>,
    pub search_history: Vec<String>,
    pub history_idx: usize,
    pub pending_cmd: String,
    pub search: bool,
    pub regex: bool,
    pub search_text: String,
    pub mode: Mode,
    pub theme: EditorTheme,
    pub wrap: bool,
    pub syntax: bool,
    pub show_line_numbers: bool,
}

impl Editor {
    pub fn new(file_paths: Vec<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut buffer_manager = BufferManager::new();
        for path in file_paths {
            buffer_manager.add_buffer(EditorBuffer::new(&path)?);
        }

        if buffer_manager.buffers.is_empty() {
            buffer_manager.add_buffer(EditorBuffer::new("")?);
        }

        let cmd = Document::new("")?;
        let theme = EditorTheme::from_highlights(&buffer_manager.active().hl);

        Ok(Self {
            buffer_manager,
            cmd,
            command_history: Vec::new(),
            search_history: Vec::new(),
            history_idx: 0,
            pending_cmd: String::new(),
            search: false,
            regex: false,
            search_text: "".to_string(),
            mode: Mode::Normal,
            theme,
            wrap: false,
            syntax: true,
            show_line_numbers: false,
        })
    }
}
