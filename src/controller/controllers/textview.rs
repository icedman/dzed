use crate::controller::ControllerResult;
use crate::controller::ViewController;

use crate::editor::Editor;
use crate::editor::display::display_map::DisplayPoint;
use crate::ui::Ui;
use crate::ui::layout::Rect;
use std::io::Write;
use text::ToPoint;
use crate::controller::actions::Action;

pub struct TextViewController {}

impl TextViewController {
    pub fn new() -> Self {
        TextViewController {}
    }
}

impl ViewController for TextViewController {
    fn update(
        &self,
        editor: &mut Editor,
        ui: &Ui,
        window_id: usize,
        rect: Rect,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        let buffer = editor.buffer_manager.active_mut();

        // Update layout before wrapping so the wrap width reflects the current gutter.
        let row_count = buffer.doc.buffer().row_count();
        let gutter_width = if editor.show_line_numbers {
            2 + if row_count == 0 {
                0
            } else {
                row_count.ilog10() as usize
            }
        } else {
            0
        };

        buffer.display_map.margin_left = gutter_width as u32;
        let wrap_cols = (rect.width as i32)
            .saturating_sub(buffer.display_map.margin_left as i32)
            .saturating_sub(buffer.display_map.margin_right as i32)
            .max(1);
        buffer
            .display_map
            .set_wrap_width(editor.wrap.then_some(wrap_cols as u32));


        if editor.should_sync {
            buffer.display_map.fold(
                buffer.doc.folds.clone(),
                buffer.doc.buffer().snapshot().clone(),
            );

            let (start, _) = buffer
                .doc
                .selections()
                .rows_in_selection(buffer.doc.buffer());
            buffer.hl.invalidate_state(start);

            // Spawn background highlight task
            let hl_task_id = buffer
                .latest_hl_task_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            editor
                .services
                .background_worker
                .spawn_task(crate::services::background::BackgroundTask::Highlight {
                    owner_id: window_id,
                    file_path: buffer.file_path.clone(),
                    snapshot: buffer.doc.buffer().snapshot().clone(),
                    start_row: start,
                    row_count: buffer.doc.buffer().row_count() - start,
                    colorscheme: std::sync::Arc::new(editor.colorscheme.clone()),
                    theme: std::sync::Arc::new(editor.theme.theme.clone()),
                    use_colorscheme: editor.use_colorscheme,
                    task_id: crate::services::background::TaskId(hl_task_id),
                    latest_task_id: buffer.latest_hl_task_id.clone(),
                });

            // Spawn background wrap task
            let wrap_width = editor.wrap.then_some(wrap_cols as u32);
            let wrap_task_id = buffer
                .latest_wrap_task_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            editor
                .services
                .background_worker
                .spawn_task(crate::services::background::BackgroundTask::Wrap {
                    owner_id: window_id,
                    file_path: buffer.file_path.clone(),
                    snapshot: buffer.doc.buffer().snapshot().clone(),
                    folds: buffer.doc.folds.clone(),
                    wrap_width,
                    task_id: crate::services::background::TaskId(wrap_task_id),
                    latest_task_id: buffer.latest_wrap_task_id.clone(),
                });

            if editor.tree_sitter
                && let Some(grammar) = buffer.grammar
            {
                let parse_task_id = buffer
                    .latest_parse_task_id
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                    + 1;
                editor
                    .services
                    .background_worker
                    .spawn_task(crate::services::background::BackgroundTask::Parse {
                        owner_id: window_id,
                        file_path: buffer.file_path.clone(),
                        snapshot: buffer.doc.buffer().snapshot().clone(),
                        grammar,
                        task_id: crate::services::background::TaskId(parse_task_id),
                        latest_task_id: buffer.latest_parse_task_id.clone(),
                    });
            }

        let cursor = buffer.doc.selection();
        let cursor_point = cursor.head().to_point(buffer.doc.buffer());
        let display_cursor = buffer
            .display_map
            .snapshot()
            .point_to_display_point(cursor_point);
        buffer.display_map.scroll_to_cursor(
            display_cursor,
            rect.height as i32,
            rect.width as i32,
        );

        // highlighting code
        let display_snapshot = buffer.display_map.snapshot();
        let total_rows = display_snapshot.row_count();
        let end_line =
            (display_snapshot.scroll_y + display_snapshot.visible_rows + 4).min(total_rows);

        if editor.syntax && end_line > display_snapshot.scroll_y {
            let start_buffer_row =
                display_snapshot.buffer_row_for_display_row(display_snapshot.scroll_y);
            let last_visible_display_row = end_line.saturating_sub(1);
            let end_point = display_snapshot.display_point_to_point(DisplayPoint::new(
                last_visible_display_row,
                display_snapshot.line_len(last_visible_display_row),
            ));
            let end_buffer_row = end_point.row;
            let end_buffer_row_exclusive = end_buffer_row + 1;

            if !buffer
                .hl
                .is_sync(&buffer.doc.buffer().snapshot())
                || !buffer
                    .hl
                    .contains_rows(start_buffer_row, end_buffer_row_exclusive)
            {
                buffer.hl.highlight_lines(
                    &buffer.doc.buffer().snapshot(),
                    start_buffer_row,
                    end_buffer_row_exclusive - start_buffer_row,
                    &editor.colorscheme,
                    &editor.theme.theme,
                    editor.use_colorscheme,
                );
            }
        }

        }

        Ok(ControllerResult::None)
    }

    fn handle_action(
        &self,
        action: crate::controller::actions::Action,
        editor: &mut Editor,
        ui: &crate::ui::Ui,
        window_id: usize,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        if action != Action::NoOp {
            editor.apply_active_action(&action);
            editor.should_sync = true;
            editor.should_redraw = true;
        }
        Ok(ControllerResult::None)
    }
}
