use super::layout::Rect;
use super::view::View;
use crate::actions::Mode;
use crate::editor::Editor;
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};
use std::io::Write;
use text::{Point, ToPoint};

pub struct StatusBarView;

impl StatusBarView {
    pub fn new() -> Self {
        StatusBarView
    }
}

impl View for StatusBarView {
    fn draw(
        &mut self,
        mut w: &mut dyn Write,
        rect: Rect,
        editor: &mut Editor,
    ) -> std::io::Result<()> {
        draw_statusbar_impl(&mut w, rect, editor)
    }

    fn handle_event(
        &mut self,
        _event: &crossterm::event::Event,
        _editor: &mut Editor,
    ) -> Option<crate::input::HandleEvent> {
        None
    }
}

fn draw_statusbar_impl<W: Write>(w: &mut W, rect: Rect, editor: &Editor) -> std::io::Result<()> {
    if rect.width == 0 || rect.height == 0 {
        return Ok(());
    }

    execute!(
        w,
        SetForegroundColor(editor.theme.fg),
        SetBackgroundColor(editor.theme.gutter),
        MoveTo(rect.x, rect.y)
    )?;

    let active_buffer = editor.buffer_manager.active();
    let cursor_point = active_buffer
        .doc
        .selection()
        .head()
        .to_point(active_buffer.doc.buffer());

    let active_idx = editor.buffer_manager.active_idx;
    let buffer_count = editor.buffer_manager.buffers.len();
    let buffer = active_buffer.doc.buffer();
    let row_len = buffer.line_len(cursor_point.row as u32);
    let selection = active_buffer.doc.selection();
    let cursor_offset = buffer.offset_for_anchor(&selection.head());
    let syntax_context = editor
        .tree_sitter
        .then_some(active_buffer.syntax_tree.as_ref())
        .flatten()
        .map(|syntax_tree| {
            let node = syntax_tree
                .named_node_at_byte(cursor_offset)
                .map(|node| node.kind)
                .unwrap_or_else(|| "?".to_string());
            let scope = syntax_tree
                .current_scope(buffer.snapshot(), cursor_offset)
                .map(|scope| scope.name.unwrap_or(scope.kind))
                .unwrap_or_else(|| "-".to_string());
            format!(
                "ts:{} node:{node} scope:{scope}",
                syntax_tree.grammar().name()
            )
        })
        .unwrap_or_else(|| {
            if editor.tree_sitter {
                "ts:- node:- scope:-".to_string()
            } else {
                "ts:off".to_string()
            }
        });

    let status_text = format!(
        "[{}/{}] {} {} [seq: {}] [op: {:?}] [motion: {:?}] [action: {:?}]",
        active_idx + 1,
        buffer_count,
        active_buffer.file_path,
        editor.mode,
        editor.input.buffer,
        editor.input.resolved_op,
        editor.input.resolved_motion,
        editor.input.resolved_action,
    );

    let truncated_text: String = status_text.chars().take(rect.width as usize).collect();
    execute!(w, Print(&truncated_text))?;

    let cols_remaining = (rect.width as usize).saturating_sub(truncated_text.chars().count());
    if cols_remaining > 0 {
        execute!(w, Print(" ".repeat(cols_remaining)))?;
    }

    execute!(w, ResetColor)?;
    Ok(())
}
