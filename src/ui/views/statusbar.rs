use crate::ui::layout::Rect;
use crate::ui::views::View;
use crate::{controller::controllers::ViewController, editor::Editor};
use std::io::Write;

use collections::Equivalent;
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};

pub struct StatusBarView;

impl StatusBarView {
    pub fn new() -> Self {
        StatusBarView {}
    }
}

impl StatusBarView {
    fn draw_statusbar<W: Write>(
        &self,
        w: &mut W,
        rect: Rect,
        editor: &Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
        _doc: Option<&crate::editor::document::Document>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use text::{Point, ToOffset, ToPoint};
        use syntect::parsing::{ParseState, ScopeStack};
        use syntect::easy::ScopeRangeIterator;

        let active_buf = buffer_manager.active();
        let mut scope_str = String::new();

        if let Some(doc) = _doc {
            let anchor = doc.selections().last().unwrap().head();
            let cursor_offset = anchor.to_offset(&active_buf.buffer);
            let point = anchor.to_point(&active_buf.buffer);

            // 1. Textmate scopes
            let mut start = 0;
            let mut cached_state = None;
            for (&row, state) in doc.hl.get_state_cache() {
                if row <= point.row as usize && row >= start {
                    start = row;
                    cached_state = Some(state);
                }
            }

            let mut parser = match cached_state {
                Some(state) => state.parser_state.clone(),
                None => ParseState::new(doc.hl.syntax()),
            };
            let mut stack = match cached_state {
                Some(state) => state.scope_stack.clone().unwrap_or_else(ScopeStack::new),
                None => ScopeStack::new(),
            };

            for r in start as u32..point.row {
                let start_off = Point::new(r, 0).to_offset(&active_buf.buffer);
                let end_off = Point::new(r, active_buf.buffer.line_len(r)).to_offset(&active_buf.buffer);
                let line_str: String = active_buf.buffer.snapshot().as_rope().chunks_in_range(start_off..end_off).collect();
                let line = line_str + "\n";
                if let Ok(ops) = parser.parse_line(&line, doc.hl.syntax_set()) {
                    for (_, op) in ops {
                        let _ = stack.apply(&op);
                    }
                }
            }

            let start_off = Point::new(point.row, 0).to_offset(&active_buf.buffer);
            let end_off = Point::new(point.row, active_buf.buffer.line_len(point.row)).to_offset(&active_buf.buffer);
            let line_str: String = active_buf.buffer.snapshot().as_rope().chunks_in_range(start_off..end_off).collect();
            let line = line_str + "\n";
            if let Ok(ops) = parser.parse_line(&line, doc.hl.syntax_set()) {
                let mut target_scopes = Vec::new();
                let mut column = 0_u32;
                for (range, op) in ScopeRangeIterator::new(&ops, &line) {
                    let _ = stack.apply(&op);
                    let start_column = column;
                    let len = range.end - range.start;
                    column += len as u32;
                    if point.column >= start_column && point.column < column {
                        target_scopes = stack.as_slice().to_vec();
                        break;
                    }
                }

                if !target_scopes.is_empty() {
                    scope_str.push('[');
                    let scope_names: Vec<String> = target_scopes.iter().map(|s| s.to_string()).collect();
                    scope_str.push_str(&scope_names.join(" "));
                    scope_str.push(']');
                }
            }

            // 2. Tree-sitter node kind
            if let Some(tree) = &active_buf.syntax_tree {
                if let Some(node) = tree.node_at_byte(cursor_offset) {
                    if !scope_str.is_empty() {
                        scope_str.push(' ');
                    }
                    scope_str.push_str(&format!("(TS: {})", node.kind));
                }
            }
        }

        let last_action_str = editor.last_action.to_string();
        let total_content_len = last_action_str.len() + scope_str.len() + if scope_str.is_empty() { 0 } else { 1 };
        let remaining = rect.width.saturating_sub(total_content_len as u16);
        let spacing = " ".repeat(remaining as usize);
        let status = if scope_str.is_empty() {
            format!("{}{}", last_action_str, spacing)
        } else {
            format!("{} {}{}", last_action_str, scope_str, spacing)
        };

        execute!(
            w,
            MoveTo(rect.x, rect.y),
            SetForegroundColor(Color::Black),
            SetBackgroundColor(Color::White),
            Print(status),
            ResetColor,
        )?;
        Ok(())
    }
}

impl View for StatusBarView {
    fn draw(
        &self,
        mut w: &mut dyn Write,
        rect: Rect,
        editor: &Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
        _doc: Option<&crate::editor::document::Document>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.draw_statusbar(&mut w, rect, editor, buffer_manager, _doc)
    }
}
