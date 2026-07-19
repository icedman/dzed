mod actions;
mod display;
mod document;
mod highlight;
mod selections;

use std::{
    io::{Write, stdout},
    time::Duration,
};

use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{Clear, ClearType},
};

use text::ToPoint;

use actions::{Action, Mode};
use display::display_map::DisplayMap;
use document::{BufferText, Document};
use highlight::Highlights;

pub struct EditorBuffer {
    pub file_path: String,
    pub doc: Document,
    pub display_map: DisplayMap,
    pub hl: Highlights,
    pub scroll_x: u32,
    pub scroll_y: u32,
    pub dirty_hl: bool,
}

pub struct EditorTheme {
    pub clr_fg: crossterm::style::Color,
    pub clr_bg: crossterm::style::Color,
    pub clr_caret: crossterm::style::Color,
    pub clr_select: crossterm::style::Color,
    pub clr_gutter: crossterm::style::Color,
}

impl EditorTheme {
    pub fn from_highlights(hl: &Highlights) -> Self {
        let settings = hl.theme_settings();
        let fg = settings.foreground.unwrap();
        let bg = settings.background.unwrap();
        Self {
            clr_fg: fg.rgb(),
            clr_bg: bg.darken(10).rgb(),
            clr_caret: settings.caret.unwrap_or(fg).rgb(),
            clr_select: settings.selection.unwrap_or(bg.darken(10)).rgb(),
            clr_gutter: settings.gutter.unwrap_or(bg).rgb(),
        }
    }
}

impl EditorBuffer {
    pub fn new(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let doc = Document::new(file_path)?;
        let display_map = DisplayMap::new(doc.buffer().snapshot().clone(), None);
        let hl = Highlights::new(file_path);
        Ok(Self {
            file_path: file_path.to_string(),
            doc,
            display_map,
            hl,
            scroll_x: 0,
            scroll_y: 0,
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

fn fill_to_eol(count: usize) {
    for _ in 0..count {
        print!(" ");
    }
}

pub struct Editor {
    pub buffer_manager: BufferManager,
    pub cmd: Document,
    pub pending_cmd: String,
    pub mode: Mode,
    pub theme: EditorTheme,
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
            pending_cmd: String::new(),
            mode: Mode::Normal,
            theme,
        })
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let file_paths = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        Vec::new()
    };

    let mut editor = Editor::new(file_paths)?;
    let mut stdout = stdout();
    crossterm::terminal::enable_raw_mode().unwrap();
    execute!(
        stdout,
        crossterm::event::EnableBracketedPaste,
        Clear(ClearType::All),
        MoveTo(0, 0)
    )
    .unwrap();

    let tab_size = 4;
    execute!(stdout, crossterm::cursor::Hide).unwrap();

    let mut should_redraw = true;
    let mut prev_screen_rows = 0;
    let mut prev_screen_cols = 0;

    loop {
        // get screen dimensions
        let (screen_cols, screen_rows) = {
            let (cols, rows) = crossterm::terminal::size().unwrap();
            (cols as i32, rows as i32)
        };
        // dimensions has changed
        if prev_screen_cols != screen_cols || prev_screen_rows != screen_rows {
            should_redraw = true;
        }
        prev_screen_rows = screen_rows;
        prev_screen_cols = screen_cols;

        let active_buffer = editor.buffer_manager.active_mut();

        // update display map
        active_buffer
            .display_map
            .set_wrap_width(Some(screen_cols as u32));
        active_buffer
            .display_map
            .sync(active_buffer.doc.buffer().snapshot().clone());
        let display_snapshot = active_buffer.display_map.snapshot();

        // get cursor information
        let cursor = active_buffer.doc.selection();
        let cursor_head = cursor.head();
        let cursor_point = cursor_head.to_point(&active_buffer.doc.buffer());
        let display_cursor = display_snapshot.point_to_display_point(cursor_point);
        let cursor_row = display_cursor.row() as i32;
        let cursor_col = display_cursor.column() as i32;
        let visible_rows = screen_rows - 1;
        let visible_cols = screen_cols;

        // scroll based on cursor position
        let mut cursor_screen_row = cursor_row - active_buffer.scroll_y as i32;
        while cursor_screen_row >= visible_rows {
            active_buffer.scroll_y += 1;
            cursor_screen_row = cursor_row - active_buffer.scroll_y as i32;
        }
        while cursor_screen_row < 0 && active_buffer.scroll_y > 0 {
            active_buffer.scroll_y -= 1;
            cursor_screen_row = cursor_row - active_buffer.scroll_y as i32;
        }
        let mut cursor_screen_col = cursor_col - active_buffer.scroll_x as i32;
        while cursor_screen_col >= visible_cols {
            active_buffer.scroll_x += 1;
            cursor_screen_col = cursor_col - active_buffer.scroll_x as i32;
        }
        while cursor_screen_col < 0 && active_buffer.scroll_x > 0 {
            active_buffer.scroll_x -= 1;
            cursor_screen_col = cursor_col - active_buffer.scroll_x as i32;
        }

        //------------------
        // render
        //------------------
        if should_redraw {
            should_redraw = false;
            execute!(stdout, crossterm::cursor::Hide).unwrap();

            let buffer = active_buffer.doc.buffer();
            let total_rows = display_snapshot.row_count();
            let end_line = (active_buffer.scroll_y + visible_rows as u32).min(total_rows);

            if active_buffer.dirty_hl {
                let start_buffer_row =
                    display_snapshot.buffer_row_for_display_row(active_buffer.scroll_y);
                let end_buffer_row =
                    display_snapshot.buffer_row_for_display_row(end_line.saturating_sub(1));

                active_buffer.hl.highlight_lines(
                    active_buffer.doc.buffer(),
                    start_buffer_row as usize,
                    (end_buffer_row - start_buffer_row + 1) as usize,
                );
            }
            active_buffer.dirty_hl = true;

            let mut screen_row = 0;
            for row in active_buffer.scroll_y..end_line {
                execute!(stdout, MoveTo(0, screen_row)).unwrap();
                let text = display_snapshot.line_text(row) + " ";
                let buffer_row = display_snapshot.buffer_row_for_display_row(row);
                let buffer_range = display_snapshot.buffer_range_for_display_row(row);
                let start_col = buffer_range.start.column;

                let ranges;
                if let Some(style_cache) = active_buffer.hl.render_line(buffer_row as usize) {
                    ranges = &style_cache.styles;
                } else {
                    execute!(
                        stdout,
                        crossterm::style::SetBackgroundColor(editor.theme.clr_bg)
                    )
                    .unwrap();
                    fill_to_eol(screen_cols as usize);
                    screen_row += 1;
                    continue;
                }

                // style range
                let mut range_iter = ranges.iter();
                let mut current_range = range_iter.next();

                // Skip ranges that end before our start_col
                while let Some((_, _s, e)) = current_range {
                    if *e <= start_col {
                        current_range = range_iter.next();
                    } else {
                        break;
                    }
                }

                let mut range_remaining =
                    current_range.map_or(
                        0,
                        |(_, s, e)| {
                            if *s < start_col { e - start_col } else { e - s }
                        },
                    );
                let mut current_style = current_range.map(|(style, _, _)| style);

                let mut x_scroll = active_buffer.scroll_x;
                let mut cols_remaining = screen_cols;

                for (column, ch) in text.chars().enumerate() {
                    let rc = start_col + column as u32;

                    if range_remaining == 0 {
                        current_range = range_iter.next();
                        range_remaining = current_range.map_or(0, |(_, s, e)| e - s);
                        current_style = current_range.map(|(style, _, _)| style);
                    }

                    let mut fg = editor.theme.clr_fg.clone();
                    let mut bg = editor.theme.clr_bg.clone();

                    if let Some(style) = current_style {
                        fg = style.foreground.rgb();
                        bg = style.background.darken(10).rgb();
                    }

                    let (selected, mut selected_line, at_cursor) = active_buffer
                        .doc
                        .selections()
                        .is_selected(buffer_row, rc, &buffer);
                    if selected && (editor.mode == Mode::Visual || editor.mode == Mode::Visual_Line)
                    {
                        bg = editor.theme.clr_select;
                    }
                    selected_line = selected_line && editor.mode == Mode::Visual_Line;
                    if selected_line {
                        bg = editor.theme.clr_select;
                    }
                    if at_cursor && editor.mode != Mode::Insert && editor.mode != Mode::Command {
                        fg = editor.theme.clr_bg;
                        bg = editor.theme.clr_caret;
                    }

                    execute!(stdout, crossterm::style::SetForegroundColor(fg)).unwrap();
                    execute!(stdout, crossterm::style::SetBackgroundColor(bg)).unwrap();

                    if x_scroll > 0 {
                        x_scroll = x_scroll.saturating_sub(1);
                    } else {
                        match ch {
                            '\t' => {
                                for _i in 0..tab_size {
                                    print!(" ");
                                    if at_cursor
                                        && editor.mode != Mode::Insert
                                        && editor.mode != Mode::Command
                                    {
                                        execute!(
                                            stdout,
                                            crossterm::style::SetBackgroundColor(
                                                editor.theme.clr_bg
                                            )
                                        )
                                        .unwrap();
                                    }
                                    cols_remaining = cols_remaining.saturating_sub(1);
                                }
                            }
                            _ => {
                                print!("{}", ch);
                                cols_remaining = cols_remaining.saturating_sub(1);
                            }
                        }
                    }

                    range_remaining = range_remaining.saturating_sub(1);

                    if cols_remaining <= 0 {
                        break;
                    }
                }

                execute!(
                    stdout,
                    crossterm::style::SetBackgroundColor(editor.theme.clr_bg)
                )
                .unwrap();
                fill_to_eol(cols_remaining.max(0) as usize);

                screen_row += 1;
                if screen_row + 1 > screen_rows as u16 {
                    break;
                }
            }

            // statusbar
            {
                execute!(
                    stdout,
                    crossterm::style::SetForegroundColor(editor.theme.clr_fg)
                )
                .unwrap();
                execute!(
                    stdout,
                    crossterm::style::SetBackgroundColor(editor.theme.clr_gutter)
                )
                .unwrap();
                execute!(stdout, MoveTo(0, screen_rows as u16)).unwrap();
                fill_to_eol(screen_cols as usize);
                execute!(stdout, MoveTo(0, screen_rows as u16)).unwrap();

                if editor.mode == Mode::Command {
                    print!(
                        ":{}",
                        editor
                            .cmd
                            .buffer()
                            .row_text(editor.cmd.buffer().row_count() - 1)
                    );
                } else {
                    let active_idx = editor.buffer_manager.active_idx;
                    let buffer_count = editor.buffer_manager.buffers.len();
                    let active_buffer = editor.buffer_manager.active();
                    let row_len = active_buffer.doc.buffer().line_len(cursor_point.row as u32);

                    print!(
                        "[{}/{}] {} {} {},{} rl:{} {}",
                        active_idx + 1,
                        buffer_count,
                        active_buffer.file_path,
                        match editor.mode {
                            Mode::Normal => "NORMAL",
                            Mode::Insert => "INSERT",
                            Mode::Visual => "VISUAL",
                            Mode::Visual_Line => "V-LINE",
                            Mode::Command => "COMMAND",
                        },
                        active_buffer.doc.selection().head().offset,
                        active_buffer.doc.selection().tail().offset,
                        row_len,
                        editor.pending_cmd
                    );
                }
            }

            if editor.mode == Mode::Command {
                let cmd_text = editor
                    .cmd
                    .buffer()
                    .row_text(editor.cmd.buffer().row_count() - 1);
                let cmd_col = (cmd_text.chars().count() + 1) as u16;
                execute!(
                    stdout,
                    MoveTo(cmd_col, screen_rows as u16),
                    crossterm::cursor::SetCursorStyle::BlinkingBar,
                    crossterm::cursor::Show
                )
                .unwrap();
            } else {
                execute!(
                    stdout,
                    MoveTo(cursor_screen_col as u16, cursor_screen_row as u16),
                    match editor.mode {
                        Mode::Insert => crossterm::cursor::SetCursorStyle::BlinkingBar,
                        _ => crossterm::cursor::SetCursorStyle::BlinkingBlock,
                    },
                    crossterm::cursor::Show
                )
                .unwrap();
            }

            stdout.flush().unwrap();
        }

        //------------------
        // input
        //------------------
        if event::poll(Duration::from_millis(50))? {
            let event = event::read()?;
            if let Event::Paste(content) = &event {
                if editor.mode == Mode::Insert {
                    let active_buffer = editor.buffer_manager.active_mut();
                    active_buffer
                        .doc
                        .apply_action(&Action::InsertText(content.clone()));
                    should_redraw = true;
                }
            }

            if let Event::Key(key_event) = event {
                should_redraw = false;

                let current_mode = editor.mode.clone();

                // global actions
                match (key_event.code, key_event.modifiers) {
                    (KeyCode::Esc, _) => {
                        editor.mode = Mode::Normal;
                        should_redraw = true;
                        let active_buffer = editor.buffer_manager.active_mut();
                        if active_buffer.doc.has_selection() {
                            active_buffer.doc.apply_action(&Action::ClearCursors);
                        }
                    }
                    (KeyCode::Char('q'), KeyModifiers::CONTROL) => break,
                    _ => {}
                }

                let (count, _) = {
                    let mut count_str = String::new();
                    let mut parsing_count = true;
                    for ch in editor.pending_cmd.chars() {
                        if parsing_count && ch.is_ascii_digit() {
                            count_str.push(ch);
                        } else {
                            parsing_count = false;
                        }
                    }
                    let count = count_str.parse::<u32>().unwrap_or(1);
                    (count, ())
                };

                let select = editor.mode == Mode::Visual || editor.mode == Mode::Visual_Line;
                let move_action = match (key_event.code, key_event.modifiers) {
                    (KeyCode::Left, _) => Action::MoveLeft { select, count },
                    (KeyCode::Right, _) => Action::MoveRight { select, count },
                    (KeyCode::Up, _) => Action::MoveUp { select, count },
                    (KeyCode::Down, _) => Action::MoveDown { select, count },
                    (KeyCode::PageUp, _) => Action::MoveUp {
                        select,
                        count: (visible_rows >> 1) as u32 * count,
                    },
                    (KeyCode::PageDown, _) => Action::MoveDown {
                        select,
                        count: (visible_rows >> 1) as u32 * count,
                    },
                    (KeyCode::Home, _) => Action::MoveToStartOfLine { select },
                    (KeyCode::End, _) => Action::MoveToEndOfLine { select },
                    (KeyCode::Char('0'), _) => Action::MoveToStartOfLine { select },
                    (KeyCode::Char('$'), _) => Action::MoveToEndOfLine { select },
                    (KeyCode::Char('^'), _) => Action::MoveToStartOfLineNonSpace { select },
                    (KeyCode::Char('{'), _) => Action::MoveToPreviousParagraph { select, count },
                    (KeyCode::Char('}'), _) => Action::MoveToNextParagraph { select, count },
                    _ => Action::NoOp,
                };

                let normal_action = match (key_event.code, key_event.modifiers) {
                    (KeyCode::Esc, _) => {
                        editor.pending_cmd.clear();
                        should_redraw = true;
                        Action::NoOp
                    }
                    (KeyCode::Char('i'), _) => {
                        if editor.mode != Mode::Command {
                            editor.mode = Mode::Insert;
                            should_redraw = true;
                        }
                        Action::NoOp
                    }
                    (KeyCode::Char('v'), _) => {
                        if editor.mode != Mode::Command {
                            editor.mode = Mode::Visual;
                            should_redraw = true;
                        }
                        Action::NoOp
                    }
                    (KeyCode::Char('V'), _) => {
                        if editor.mode != Mode::Command {
                            editor.mode = Mode::Visual_Line;
                            should_redraw = true;
                        }
                        Action::NoOp
                    }
                    (KeyCode::Char(':'), _) => {
                        if editor.mode != Mode::Command {
                            editor.mode = Mode::Command;
                            should_redraw = true;
                        }
                        Action::NoOp
                    }
                    (KeyCode::Char('r'), KeyModifiers::CONTROL) => Action::Redo { count },
                    (KeyCode::Char('u'), _) => Action::Undo { count },
                    (KeyCode::Char('h'), _) => Action::MoveLeft { select, count },
                    (KeyCode::Char('l'), _) => Action::MoveRight { select, count },
                    (KeyCode::Char('k'), _) => Action::MoveUp { select, count },
                    (KeyCode::Char('j'), _) => Action::MoveDown { select, count },
                    (KeyCode::Delete, _) => Action::DeleteText {
                        count: count as usize,
                    },
                    (KeyCode::Backspace, _) => Action::MoveLeft { select, count },
                    (KeyCode::Left, KeyModifiers::SHIFT) => Action::MoveToPreviousWord {
                        select: false,
                        count,
                    },
                    (KeyCode::Right, KeyModifiers::SHIFT) => Action::MoveToNextWord {
                        select: false,
                        count,
                    },
                    (KeyCode::Char(c), _) => {
                        editor.pending_cmd.push(c);
                        let (count, cmd_without_count) = {
                            let mut count_str = String::new();
                            let mut cmd_str = String::new();
                            let mut parsing_count = true;
                            for ch in editor.pending_cmd.chars() {
                                if parsing_count
                                    && ch.is_ascii_digit()
                                    && (ch != '0' || !count_str.is_empty())
                                {
                                    count_str.push(ch);
                                } else {
                                    parsing_count = false;
                                    cmd_str.push(ch);
                                }
                            }
                            let count = if count_str.is_empty() {
                                1
                            } else {
                                count_str.parse::<u32>().unwrap_or(1)
                            };
                            (count, cmd_str)
                        };

                        let action = match cmd_without_count.as_str() {
                            "gg" => Some(Action::MoveToStartOfDocument { select }),
                            "G" => Some(Action::MoveToEndOfDocument { select }),
                            "dd" => Some(Action::DeleteCurrentLine { count }),
                            "dw" => Some(Action::DeleteMotion {
                                count,
                                motion: Box::new(Action::MoveToNextWord {
                                    select: true,
                                    count: 1,
                                }),
                            }),
                            "db" => Some(Action::DeleteMotion {
                                count,
                                motion: Box::new(Action::MoveToPreviousWord {
                                    select: true,
                                    count: 1,
                                }),
                            }),
                            "de" => Some(Action::DeleteMotion {
                                count,
                                motion: Box::new(Action::MoveToNextWordEnd {
                                    select: true,
                                    count: 1,
                                }),
                            }),
                            "dge" => Some(Action::DeleteMotion {
                                count,
                                motion: Box::new(Action::MoveToPreviousWordEnd {
                                    select: true,
                                    count: 1,
                                }),
                            }),
                            "dj" => Some(Action::DeleteMotion {
                                count,
                                motion: Box::new(Action::MoveDown {
                                    select: true,
                                    count: 1,
                                }),
                            }),
                            "dk" => Some(Action::DeleteMotion {
                                count,
                                motion: Box::new(Action::MoveUp {
                                    select: true,
                                    count: 1,
                                }),
                            }),
                            "dh" => Some(Action::DeleteMotion {
                                count,
                                motion: Box::new(Action::MoveLeft {
                                    select: true,
                                    count: 1,
                                }),
                            }),
                            "dl" => Some(Action::DeleteMotion {
                                count,
                                motion: Box::new(Action::MoveRight {
                                    select: true,
                                    count: 1,
                                }),
                            }),
                            "d0" => Some(Action::DeleteMotion {
                                count,
                                motion: Box::new(Action::MoveToStartOfLine { select: true }),
                            }),
                            "d$" => Some(Action::DeleteMotion {
                                count,
                                motion: Box::new(Action::MoveToEndOfLine { select: true }),
                            }),
                            "d^" => Some(Action::DeleteMotion {
                                count,
                                motion: Box::new(Action::MoveToStartOfLineNonSpace {
                                    select: true,
                                }),
                            }),
                            "d{" => Some(Action::DeleteMotion {
                                count,
                                motion: Box::new(Action::MoveToPreviousParagraph {
                                    select: true,
                                    count: 1,
                                }),
                            }),
                            "d}" => Some(Action::DeleteMotion {
                                count,
                                motion: Box::new(Action::MoveToNextParagraph {
                                    select: true,
                                    count: 1,
                                }),
                            }),
                            "x" => Some(Action::Delete { count }),
                            "b" => Some(Action::MoveToPreviousWord { select, count }),
                            "w" => Some(Action::MoveToNextWord { select, count }),
                            "e" => Some(Action::MoveToNextWordEnd { select, count }),
                            "ge" => Some(Action::MoveToPreviousWordEnd { select, count }),
                            s if s.starts_with('f') && s.len() == 2 => {
                                let ch = s.chars().nth(1).unwrap();
                                Some(Action::FindCharacter {
                                    select,
                                    count,
                                    char: ch,
                                    forward: true,
                                })
                            }
                            s if s.starts_with('F') && s.len() == 2 => {
                                let ch = s.chars().nth(1).unwrap();
                                Some(Action::FindCharacter {
                                    select,
                                    count,
                                    char: ch,
                                    forward: false,
                                })
                            }
                            _ => None,
                        };

                        if let Some(a) = action {
                            editor.pending_cmd.clear();
                            a
                        } else {
                            Action::NoOp
                        }
                    }
                    _ => Action::NoOp,
                };

                let _visual_action = Action::NoOp;

                // document actions
                let insert_action = match (key_event.code, key_event.modifiers) {
                    (KeyCode::Enter, _) if editor.mode == Mode::Insert => Action::InsertNewLine,
                    (KeyCode::Tab, _)
                        if editor.mode == Mode::Insert || editor.mode == Mode::Command =>
                    {
                        Action::InsertTab
                    }
                    (KeyCode::Delete, _)
                        if editor.mode == Mode::Insert || editor.mode == Mode::Command =>
                    {
                        Action::Delete { count: 1 }
                    }
                    (KeyCode::Backspace, _)
                        if editor.mode == Mode::Insert || editor.mode == Mode::Command =>
                    {
                        Action::Backspace
                    }
                    (KeyCode::Char(c), _)
                        if editor.mode == Mode::Insert || editor.mode == Mode::Command =>
                    {
                        Action::InsertText(c.to_string())
                    }
                    _ => Action::NoOp,
                };

                let _command_action = if editor.mode != Mode::Insert {
                    match (key_event.code, key_event.modifiers) {
                        _ => Action::NoOp,
                    }
                } else {
                    Action::NoOp
                };

                // loop!
                match current_mode {
                    Mode::Normal => {
                        if normal_action != Action::NoOp {
                            let active_buffer = editor.buffer_manager.active_mut();
                            active_buffer.doc.apply_action(&normal_action);
                            editor.pending_cmd.clear();
                        } else if move_action != Action::NoOp {
                            let active_buffer = editor.buffer_manager.active_mut();
                            active_buffer.doc.apply_action(&move_action);
                            editor.pending_cmd.clear();
                        }
                    }
                    Mode::Visual => {
                        if normal_action != Action::NoOp {
                            let active_buffer = editor.buffer_manager.active_mut();
                            active_buffer.doc.apply_action(&normal_action);
                            editor.pending_cmd.clear();
                        } else if move_action != Action::NoOp {
                            let active_buffer = editor.buffer_manager.active_mut();
                            active_buffer.doc.apply_action(&move_action);
                            editor.pending_cmd.clear();
                        }
                    }
                    Mode::Visual_Line => {
                        if normal_action != Action::NoOp {
                            let active_buffer = editor.buffer_manager.active_mut();
                            active_buffer.doc.apply_action(&normal_action);
                            editor.pending_cmd.clear();
                        } else if move_action != Action::NoOp {
                            let active_buffer = editor.buffer_manager.active_mut();
                            active_buffer.doc.apply_action(&move_action);
                            editor.pending_cmd.clear();
                        }
                    }
                    Mode::Insert => {
                        if insert_action != Action::NoOp {
                            let active_buffer = editor.buffer_manager.active_mut();
                            active_buffer.doc.apply_action(&insert_action);
                        } else if move_action != Action::NoOp {
                            let active_buffer = editor.buffer_manager.active_mut();
                            active_buffer.doc.apply_action(&move_action);
                            editor.pending_cmd.clear();
                        }
                    }
                    Mode::Command => {
                        if let (KeyCode::Enter, _) = (key_event.code, key_event.modifiers) {
                            let command_text = editor.cmd.buffer().row_text(0);
                            let command_parts: Vec<&str> =
                                command_text.trim().split_whitespace().collect();

                            if !command_parts.is_empty() {
                                match command_parts[0] {
                                    "q" => break,
                                    "bn" => {
                                        editor.buffer_manager.switch_next();
                                        editor.theme = EditorTheme::from_highlights(
                                            &editor.buffer_manager.active().hl,
                                        );
                                    }
                                    "bp" => {
                                        editor.buffer_manager.switch_prev();
                                        editor.theme = EditorTheme::from_highlights(
                                            &editor.buffer_manager.active().hl,
                                        );
                                    }
                                    "e" if command_parts.len() > 1 => {
                                        if let Ok(new_buffer) = EditorBuffer::new(command_parts[1])
                                        {
                                            editor.buffer_manager.add_buffer(new_buffer);
                                            editor.theme = EditorTheme::from_highlights(
                                                &editor.buffer_manager.active().hl,
                                            );
                                        }
                                    }
                                    cmd if cmd.parse::<u32>().is_ok() => {
                                        let line_number = cmd.parse::<u32>().unwrap();
                                        let active_buffer = editor.buffer_manager.active_mut();
                                        active_buffer.doc.apply_action(&Action::MoveToLine {
                                            select: false,
                                            line: line_number,
                                        });
                                    }
                                    _ => {}
                                }
                            }

                            // Clear command buffer and return to Normal mode
                            editor.cmd = Document::new("").unwrap();
                            editor.mode = Mode::Normal;
                            should_redraw = true;
                        } else if insert_action != Action::NoOp {
                            editor.cmd.apply_action(&insert_action);
                        } else if let (KeyCode::Backspace, _) =
                            (key_event.code, key_event.modifiers)
                        {
                            editor.cmd.apply_action(&Action::Backspace);
                        }
                    }
                }

                if !should_redraw {
                    if normal_action != Action::NoOp
                        || move_action != Action::NoOp
                        || insert_action != Action::NoOp
                    {
                        should_redraw = true;
                    }
                }
            }
        }
    }

    crossterm::terminal::disable_raw_mode().unwrap();
    execute!(
        stdout,
        crossterm::event::DisableBracketedPaste,
        Clear(ClearType::All),
        MoveTo(0, 0)
    )
    .unwrap();
    execute!(stdout, crossterm::cursor::Show).unwrap();

    Ok(())
}
