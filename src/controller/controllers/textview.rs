use crate::controller::ControllerResult;
use crate::controller::ViewController;
use crate::controller::actions::Action;
use crate::editor::Editor;
use crate::editor::display::display_map::DisplayPoint;
use crate::ui::Ui;
use crate::ui::layout::Rect;
use crate::services::background::{self, BackgroundTask, TaskId};
use text::ToPoint;

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
        buffer_manager: &mut crate::editor::buffers::BufferManager,
        ui: &mut crate::ui::Ui,
        window_id: usize,
        rect: Rect,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        let window = ui.windows.get_mut(&window_id).unwrap();
        let document = window.doc.as_mut().unwrap();
        let buffer = buffer_manager.find_mut(document).unwrap();

        // Update layout before wrapping so the wrap width reflects the current gutter.
        let row_count = buffer.buffer.row_count();
        let gutter_width = if editor.show_line_numbers {
            2 + if row_count == 0 {
                0
            } else {
                row_count.ilog10() as usize
            }
        } else {
            0
        };

        document.display_map.margin_left = gutter_width as u32;
        let wrap_cols = (rect.width as i32)
            .saturating_sub(document.display_map.margin_left as i32)
            .saturating_sub(document.display_map.margin_right as i32)
            .max(1);
        document
            .display_map
            .set_wrap_width(editor.wrap.then_some(wrap_cols as u32));

        if document.should_sync {
            let snapshot = buffer.buffer.snapshot().clone();
            document.display_map.fold(
                document.folds.clone(),
                snapshot.clone(),
            );

            let text_changed = !document.hl.is_sync(&snapshot);
            let wrap_width = editor.wrap.then_some(wrap_cols as u32);
            let wrap_changed = text_changed || document.display_map.wrap_width != wrap_width;

            if text_changed {
                let (start, _) = document
                    .selections()
                    .rows_in_selection(&buffer.buffer);
                document.hl.invalidate_state(start);

                // Spawn background highlight task
                let display_snapshot = document.display_map.snapshot();
                let total_rows = display_snapshot.row_count();
                let visible_start = display_snapshot.scroll_y;
                let visible_end = (visible_start + display_snapshot.visible_rows + 4).min(total_rows);
                let start_buffer_row = display_snapshot.buffer_row_for_display_row(visible_start);
                let end_buffer_row = display_snapshot.buffer_row_for_display_row(visible_end.saturating_sub(1));

                let hl_start = start_buffer_row.saturating_sub(100).min(start);
                let hl_end = (end_buffer_row + 100).max(start + 1).min(buffer.buffer.row_count());

                let hl_task_id = document
                    .latest_hl_task_id
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                    + 1;
                editor.services.background_worker.spawn_task(
                    BackgroundTask::Highlight {
                        owner_id: window_id,
                        file_path: buffer.file_path.clone(),
                        snapshot: snapshot.clone(),
                        start_row: hl_start,
                        row_count: hl_end - hl_start,
                        colorscheme: std::sync::Arc::new(editor.colorscheme.clone()),
                        syntax_tree: buffer.syntax_tree.clone(),
                        textmate_highlights: editor.textmate_highlights,
                        treesitter_highlights: editor.treesitter_highlights,
                        map_scope_to_scheme: editor.map_scope_to_scheme,
                        task_id: TaskId(hl_task_id),
                        latest_task_id: document.latest_hl_task_id.clone(),
                    },
                );

                if editor.tree_sitter
                    && let Some(grammar) = buffer.grammar
                {
                    let parse_task_id = document
                        .latest_parse_task_id
                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                        + 1;
                    editor.services.background_worker.spawn_task(
                        BackgroundTask::Parse {
                            owner_id: window_id,
                            file_path: buffer.file_path.clone(),
                            snapshot: snapshot.clone(),
                            grammar,
                            task_id: TaskId(parse_task_id),
                            latest_task_id: document.latest_parse_task_id.clone(),
                        },
                    );
                }
            }

            if wrap_changed {
                // Spawn background wrap task
                let wrap_task_id = document
                    .latest_wrap_task_id
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                    + 1;
                editor.services.background_worker.spawn_task(
                    BackgroundTask::Wrap {
                        owner_id: window_id,
                        file_path: buffer.file_path.clone(),
                        snapshot: snapshot.clone(),
                        folds: document.folds.clone(),
                        wrap_width,
                        task_id: TaskId(wrap_task_id),
                        latest_task_id: document.latest_wrap_task_id.clone(),
                    },
                );
            }

            let cursor = document.selection();
            let cursor_point = cursor.head().to_point(&buffer.buffer);
            let display_cursor = document
                .display_map
                .snapshot()
                .point_to_display_point(cursor_point);
            document.display_map.scroll_to_cursor(
                display_cursor,
                rect.height as i32,
                rect.width as i32,
            );

            // highlighting code
            let display_snapshot = document.display_map.snapshot();
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

                let snapshot = buffer.buffer.snapshot().clone();
                if !document.hl.is_sync(&snapshot)
                    || !document
                        .hl
                        .contains_rows(start_buffer_row, end_buffer_row_exclusive)
                {
                    document.hl.highlight_lines(
                        &snapshot,
                        start_buffer_row,
                        end_buffer_row_exclusive - start_buffer_row,
                        &editor.colorscheme,
                        buffer.syntax_tree.as_ref(),
                        editor.textmate_highlights,
                        editor.treesitter_highlights,
                        editor.map_scope_to_scheme,
                    );
                }
            }
            document.should_sync = false;
        }

        Ok(ControllerResult::None)
    }

    fn handle_action(
        &self,
        action: Action,
        editor: &mut Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
        ui: &mut crate::ui::Ui,
        window_id: usize,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        if action != Action::NoOp {
            editor.apply_active_action(ui, buffer_manager, &action);
            if let Some(window) = ui.windows.get_mut(&window_id) {
                if let Some(ref mut document) = window.doc {
                    document.should_sync = true;
                }
            }
            editor.should_redraw = true;
        }
        Ok(ControllerResult::None)
    }

    fn handle_task(
        &mut self,
        result: &background::BackgroundResult,
        editor: &mut Editor,
        buffer_manager: &mut crate::editor::buffers::BufferManager,
        document: Option<&mut crate::editor::document::Document>,
    ) -> Result<ControllerResult, Box<dyn std::error::Error>> {
        let document = document.expect("TextViewController requires a Document view state");
        match result {
            background::BackgroundResult::HighlightComplete {
                file_path,
                style_cache,
                task_id,
                ..
            } => {
                if let Some(buf) = buffer_manager
                    .buffers
                    .iter_mut()
                    .find(|b| &b.file_path == file_path)
                {
                    if *task_id >= background::TaskId(document.current_hl_task_id) {
                        document.current_hl_task_id = task_id.0;
                        document.hl
                            .merge_caches(style_cache.clone(), std::collections::HashMap::new());
                        document.hl.last_snapshot_version = Some(buf.buffer.snapshot().version.clone());
                        editor.should_redraw = true;
                    }
                }
            }
            background::BackgroundResult::WrapComplete {
                wrap_snapshot,
                task_id,
                ..
            } => {
                if *task_id >= background::TaskId(document.current_wrap_task_id) {
                    document.current_wrap_task_id = task_id.0;
                    document.display_map.apply_wrap_snapshot(wrap_snapshot.clone());
                    editor.should_redraw = true;
                }
            }
            background::BackgroundResult::ParseComplete {
                file_path,
                syntax_tree,
                task_id,
                ..
            } => {
                if editor.tree_sitter {
                    if let Some(buf) = buffer_manager
                        .buffers
                        .iter_mut()
                        .find(|b| &b.file_path == file_path)
                    {
                        if *task_id >= background::TaskId(document.current_parse_task_id) {
                            document.current_parse_task_id = task_id.0;
                            buf.syntax_tree = Some(syntax_tree.clone());
                            document.hl.update_treesitter_highlights(
                                &buf.buffer.snapshot(),
                                Some(&syntax_tree),
                                &editor.colorscheme,
                                editor.treesitter_highlights,
                            );
                            editor.should_redraw = true;
                        }
                    }
                }
            }
        }

        Ok(ControllerResult::None)
    }
}

