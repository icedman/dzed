use crate::actions::{Action, Mode};
use crate::clipboard::ClipboardKind;
use crate::editor::Editor;
use crate::selections::{Motions, SelectionCollection};

use clock::ReplicaId;
use rope::Point;
use std::{cmp::Ordering, io};
use sum_tree::Bias;
use text::{Anchor, Buffer, BufferId, BufferSnapshot, Selection, SelectionGoal, ToOffset, ToPoint};

pub trait BufferText {
    fn row_text(&self, row: u32) -> String;
}

impl BufferText for Buffer {
    fn row_text(&self, row: u32) -> String {
        let start = Point::new(row, 0).to_offset(self);
        let end = Point::new(row, self.line_len(row)).to_offset(self);
        self.as_rope().chunks_in_range(start..end).collect()
    }
}

impl BufferText for BufferSnapshot {
    fn row_text(&self, row: u32) -> String {
        let start = Point::new(row, 0).to_offset(self);
        let end = Point::new(row, self.line_len(row)).to_offset(self);
        self.as_rope().chunks_in_range(start..end).collect()
    }
}

pub struct Document {
    buffer: Buffer,
    selections: SelectionCollection,
    mode: Mode,
    pub folds: Vec<crate::display::fold_map::Fold>,
}

impl Document {
    pub fn new_with_text(contents: &str) -> Self {
        let buffer = Buffer::new(
            ReplicaId::default(),
            BufferId::new(1).unwrap(),
            contents.to_string(),
        );
        let mut selections = SelectionCollection::new();
        selections.add(&buffer, 0);

        Self {
            buffer,
            selections,
            mode: Mode::Normal,
            folds: Vec::new(),
        }
    }

    pub fn new(file_path: &str) -> io::Result<Self> {
        let contents = if std::path::Path::new(file_path).exists() {
            match std::fs::read_to_string(file_path) {
                Ok(s) => s,
                Err(_) => "File not found".to_string(),
            }
        } else {
            "".to_string()
        };
        let buffer = Buffer::new(ReplicaId::default(), BufferId::new(1).unwrap(), contents);
        let mut selections = SelectionCollection::new();
        selections.add(&buffer, 0);

        Ok(Self {
            buffer,
            selections,
            mode: Mode::Normal,
            folds: Vec::new(),
        })
    }

    pub fn new_line(&self) -> &str {
        self.buffer.line_ending().as_str()
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn undo(&mut self, count: u32) {
        for _ in 0..count {
            self.buffer.undo();
        }
    }

    pub fn redo(&mut self, count: u32) {
        for _ in 0..count {
            self.buffer.redo();
        }
    }

    pub fn fold(&mut self, _count: u32, editor: &Editor) {
        let active_idx = editor.buffer_manager.active_idx;
        let buffer = &editor.buffer_manager.buffers[active_idx];
        if let Some(syntax_tree) = &buffer.syntax_tree {
            let mut seen_ranges = std::collections::HashSet::new();
            for selection in self.selections.selections.iter() {
                let head_point = selection.head().to_point(&self.buffer);
                let head_offset = head_point.to_offset(&self.buffer);
                if let Some(block) = syntax_tree.enclosing_block_at_byte(head_offset) {
                    if !editor.fold_multiline_only || block.end_position.row > block.start_position.row {
                        let mut start_offset = block.byte_range.start;
                        let mut end_offset = block.byte_range.end;
                        
                        let first_char = self.buffer.text_for_range(start_offset..start_offset + 1).next().and_then(|s| s.chars().next());
                        let last_char = if end_offset > 0 {
                            self.buffer.text_for_range(end_offset - 1..end_offset).next().and_then(|s| s.chars().next())
                        } else {
                            None
                        };

                        if let (Some(fc), Some(lc)) = (first_char, last_char) {
                            if (fc == '{' && lc == '}') || (fc == '[' && lc == ']') || (fc == '(' && lc == ')') {
                                start_offset += 1;
                                end_offset -= 1;
                            }
                        }

                        let range = block.byte_range.clone();
                        if seen_ranges.insert(range) {
                            let fold = crate::display::fold_map::Fold {
                                start: start_offset.to_point(&self.buffer),
                                end: end_offset.to_point(&self.buffer),
                            };
                            if !self.folds.contains(&fold) {
                                self.folds.push(fold);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn unfold(&mut self, _count: u32, editor: &Editor) {
        let mut to_remove = Vec::new();
        for selection in self.selections.selections.iter() {
            let head_point = selection.head().to_point(&self.buffer);
            for (idx, fold) in self.folds.iter().enumerate() {
                if head_point >= fold.start && head_point <= fold.end {
                    to_remove.push(idx);
                }
            }
        }
        to_remove.sort_unstable();
        to_remove.dedup();
        for idx in to_remove.into_iter().rev() {
            self.folds.remove(idx);
        }
    }

    pub fn snap_selections_to_folds(&mut self, action: &Action) {
        if self.folds.is_empty() {
            return;
        }

        // Detect direction based on motion/action
        let moving_right = match action {
            Action::MoveRight { .. }
            | Action::MoveDown { .. }
            | Action::MoveToWord { .. }
            | Action::MoveToWordEnd { .. }
            | Action::MoveToBigWord { .. }
            | Action::MoveToEndOfLine { .. }
            | Action::MoveToEndOfDocument { .. }
            | Action::MoveToEndOfNextLine { .. } => true,
            _ => false,
        };

        let mut updated_selections = Vec::new();
        for selection in &self.selections.selections {
            let head = selection.head().to_point(&self.buffer);
            let mut new_head = head;
            for fold in &self.folds {
                if head > fold.start && head < fold.end {
                    new_head = if moving_right { fold.end } else { fold.start };
                    break;
                }
            }

            if new_head != head {
                let anchor_pos = selection.tail().to_point(&self.buffer);
                let mut new_anchor = anchor_pos;
                if anchor_pos == head {
                    new_anchor = new_head;
                }

                let new_sel = Selection {
                    id: selection.id,
                    start: self.buffer.anchor_at(&new_anchor, Bias::Left),
                    end: self.buffer.anchor_at(&new_head, Bias::Left),
                    reversed: new_head < new_anchor,
                    goal: selection.goal,
                };
                updated_selections.push(new_sel);
            }
        }

        for new_sel in updated_selections {
            self.selections.update(&self.buffer, &new_sel);
        }

        if let Some(first) = self.selections.first() {
            self.selections.point = first.head().to_point(&self.buffer);
        }
    }

    pub fn enter_mode(&mut self, mode: Mode) {
        if self.mode == mode {
            self.clear_selections();
            return;
        }

        if self.mode == Mode::VisualBlock {
            self.selections.end_block();
        }
        if self.mode == Mode::VisualLine {
            self.selections.end_line();
        }

        self.mode = mode;

        if self.mode == Mode::VisualBlock {
            self.selections.begin_block(&self.buffer);
        }
        if self.mode == Mode::VisualLine {
            self.selections.begin_line(&self.buffer);
        }
    }

    pub fn current_mode(&self) -> Mode {
        return self.mode.clone();
    }

    pub fn sync(&mut self) {
        if self.mode == Mode::VisualBlock {
            self.selections.sync_block(&self.buffer);
        }
        if self.mode == Mode::VisualLine {
            self.selections.sync_line(&self.buffer);
        }
    }

    pub fn select_similar(&mut self) {
        // if !self.has_selection() {
        //    self.select_in(&SelectInKind::Word);
        // } else {
        let cursor = self.selection();
        let selected_text = cursor.text(&self.buffer);
        if let Some(mut next_match) = cursor.clone().move_to_next_match_within(
            selected_text.as_str(),
            &self.buffer,
            self.buffer.row_count(),
        ) {
            for _ in 0..selected_text.len().saturating_sub(1) {
                next_match = next_match.move_right_once(true, &self.buffer);
            }

            let next_cursor = Selection {
                id: cursor.id,
                start: next_match.head(),
                end: next_match.tail(),
                reversed: false,
                goal: SelectionGoal::None,
            };
            if self
                .selections
                .has_similar_cursor(&next_cursor, &self.buffer)
            {
                return;
            }

            let sel = self.add_selection();
            self.selections.update(
                &self.buffer,
                &Selection {
                    id: sel.id,
                    start: cursor.head(),
                    end: cursor.tail(),
                    reversed: false,
                    goal: SelectionGoal::None,
                },
            );
            self.selections.update(&self.buffer, &next_cursor);
        }
        // }
    }

    pub fn apply_action(&mut self, action: &Action, editor: &Editor) {
        let mut action_owned = action.clone();
        if self.mode.is_visual() {
            action_owned = action_owned.with_select(true);
        }
        let action = &action_owned;

        let mut next_action = Action::NoOp;
        match action {
            Action::InsertNewLineMotion { .. }
            | Action::Change { .. }
            | Action::ChangeLine { .. }
            | Action::ChangeMotion { .. } => next_action = Action::SetToInsert,
            _ => {}
        }

        // These actions immediately elevates mode to Insert
        if self.mode == Mode::VisualBlock {
            match action {
                Action::Delete { .. } | Action::DeleteMotion { .. } => {
                    next_action = Action::SetToInsert
                }
                _ => {}
            }
        }
        // These actions immediately drops mode back to Normal
        if self.mode.is_visual() {
            match action {
                Action::Yank { .. } | Action::YankLine { .. } | Action::YankMotion { .. } => {
                    next_action = Action::SetToNormal
                }
                _ => {}
            }
        }

        match action {
            Action::Clear => {
                self.clear_selections();
                self.enter_mode(Mode::Normal);
                return;
            }
            Action::SetToNormal => {
                self.enter_mode(Mode::Normal);
                return;
            }
            Action::SetToInsert => {
                self.enter_mode(Mode::Insert);
                return;
            }
            Action::SetToAppend => {
                let cursors = self.selections.selections.clone();
                for cursor in cursors.iter() {
                    let point = cursor.head().to_point(&self.buffer);
                    let row_len = self.buffer.line_len(point.row);
                    if point.column < row_len {
                        self.selections.move_right(false, 1, &self.buffer);
                    }
                }
                self.enter_mode(Mode::Insert);
                return;
            }
            Action::SetToAppendEndOfLine => {
                self.selections.move_to_end_of_line(false, &self.buffer);
                self.enter_mode(Mode::Insert);
                return;
            }
            Action::SetToOpenLineBelow { count } => {
                let count = *count;
                self.selections.move_to_end_of_line(false, &self.buffer);
                let current_row = self
                    .selections
                    .first()
                    .unwrap()
                    .head()
                    .to_point(&self.buffer)
                    .row;
                for _ in 0..count {
                    self.insert_text(&self.new_line().to_string());
                }
                let target_point = Point {
                    row: current_row + 1,
                    column: 0,
                };
                let target_anchor = self
                    .buffer
                    .anchor_at(target_point.to_offset(&self.buffer), Bias::Left);
                self.selections.clear(&self.buffer);
                let first = self.selections.first().unwrap().clone();
                let next = Selection {
                    id: first.id,
                    start: target_anchor.clone(),
                    end: target_anchor,
                    reversed: false,
                    goal: SelectionGoal::None,
                };
                self.selections.point = target_point;
                self.selections.update(&self.buffer, &next);
                self.enter_mode(Mode::Insert);
                return;
            }
            Action::SetToOpenLineAbove { count } => {
                let count = *count;
                self.selections.move_to_start_of_line(false, &self.buffer);
                let current_row = self
                    .selections
                    .first()
                    .unwrap()
                    .head()
                    .to_point(&self.buffer)
                    .row;
                for _ in 0..count {
                    self.insert_text(&self.new_line().to_string());
                }
                let target_point = Point {
                    row: current_row,
                    column: 0,
                };
                let target_anchor = self
                    .buffer
                    .anchor_at(target_point.to_offset(&self.buffer), Bias::Left);
                self.selections.clear(&self.buffer);
                let first = self.selections.first().unwrap().clone();
                let next = Selection {
                    id: first.id,
                    start: target_anchor.clone(),
                    end: target_anchor,
                    reversed: false,
                    goal: SelectionGoal::None,
                };
                self.selections.point = target_point;
                self.selections.update(&self.buffer, &next);
                self.enter_mode(Mode::Insert);
                return;
            }
            Action::SetToVisual => {
                self.enter_mode(Mode::Visual);
                return;
            }
            Action::SetToInsertStartOfLineNonSpace => {
                self.selections
                    .move_to_start_of_line_non_space(false, &self.buffer);
                self.enter_mode(Mode::Insert);
                return;
            }
            Action::SetToVisualLine => {
                self.enter_mode(Mode::VisualLine);
                return;
            }
            Action::SetToVisualBlock => {
                self.enter_mode(Mode::VisualBlock);
                return;
            }
            Action::SetToCommand => {
                self.enter_mode(Mode::Command);
                return;
            }
            Action::MoveLeft { count, select } => {
                self.selections.move_left(*select, *count, &self.buffer);
            }
            Action::MoveRight { count, select } => {
                self.selections.move_right(*select, *count, &self.buffer);
            }
            Action::MoveUp { count, select } => {
                self.selections.move_up(*select, *count, &self.buffer);
            }
            Action::MoveDown { count, select } => {
                self.selections.move_down(*select, *count, &self.buffer);
            }
            Action::MoveToPreviousWord { select, count } => {
                self.selections
                    .move_to_previous_word(*select, *count, &self.buffer)
            }
            Action::MoveToWord { select, count } => {
                self.selections
                    .move_to_next_word(*select, *count, &self.buffer)
            }
            Action::MoveToPreviousWordEnd { select, count } => self
                .selections
                .move_to_previous_word_end(*select, *count, &self.buffer),
            Action::MoveToWordEnd { select, count } => {
                self.selections
                    .move_to_word_end(*select, *count, &self.buffer)
            }
            Action::MoveToBigWord { select, count } => {
                self.selections
                    .move_to_big_word(*select, *count, &self.buffer)
            }
            Action::MoveToPreviousBigWord { select, count } => self
                .selections
                .move_to_previous_big_word(*select, *count, &self.buffer),
            Action::MoveToBigWordEnd { select, count } => {
                self.selections
                    .move_to_big_word_end(*select, *count, &self.buffer)
            }
            Action::MoveToPreviousBigWordEnd { select, count } => self
                .selections
                .move_to_previous_big_word_end(*select, *count, &self.buffer),
            Action::MoveToPreviousParagraph { select, count } => self
                .selections
                .move_to_previous_paragraph(*select, *count, &self.buffer),
            Action::MoveToNextParagraph { select, count } => self
                .selections
                .move_to_next_paragraph(*select, *count, &self.buffer),
            Action::MoveToPreviousCharacter {
                select,
                count,
                ch,
                till,
            } => self
                .selections
                .find_character(*select, *count, *ch, false, *till, &self.buffer),
            Action::MoveToNextCharacter {
                select,
                count,
                ch,
                till,
            } => self
                .selections
                .find_character(*select, *count, *ch, true, *till, &self.buffer),
            Action::SearchBackward { count } => {
                // TODO: handle count
                for _ in 0..*count {
                    self.selections
                        .move_to_previous_match("", false, &self.buffer);
                }
            }
            Action::SearchForward { count } => {
                // TODO: handle count
                for _ in 0..*count {
                    self.selections.move_to_next_match("", false, &self.buffer);
                }
            }
            Action::MoveWithinCharacter { count, ch } => {
                let select = self.current_mode().is_visual();
                let cursors = self.selections.selections.clone();
                for cursor in cursors.iter() {
                    let mut updated = false;
                    if *ch == 'w' {
                        let start_sel = cursor.move_to_word(false, &self.buffer);
                        let end_sel = cursor.move_to_word_end(false, &self.buffer);
                        let next = Selection {
                            id: cursor.id,
                            start: start_sel.head(),
                            end: end_sel.head(),
                            reversed: false,
                            goal: SelectionGoal::None,
                        };
                        self.selections.update(&self.buffer, &next);
                        updated = true;
                    } else if *ch == 'p' {
                        let prev_p = cursor.move_to_previous_paragraph(false, &self.buffer).head().to_point(&self.buffer);
                        let next_p = cursor.move_to_next_paragraph(false, &self.buffer).head().to_point(&self.buffer);
                        let start_row = if prev_p.row < self.buffer.row_count() && self.buffer.line_len(prev_p.row) == 0 {
                            prev_p.row + 1
                        } else {
                            prev_p.row
                        };
                        let end_row = if next_p.row > 0 && self.buffer.line_len(next_p.row) == 0 {
                            next_p.row - 1
                        } else {
                            next_p.row
                        };
                        let start_offset = Point { row: start_row, column: 0 }.to_offset(&self.buffer);
                        let end_offset = Point { row: end_row, column: self.buffer.line_len(end_row) }.to_offset(&self.buffer).saturating_sub(1);
                        let next = Selection {
                            id: cursor.id,
                            start: self.buffer.anchor_at(start_offset, Bias::Left),
                            end: self.buffer.anchor_at(end_offset, Bias::Right),
                            reversed: false,
                            goal: SelectionGoal::None,
                        };
                        self.selections.update(&self.buffer, &next);
                        updated = true;
                    } else if editor.tree_sitter {
                        if let Some(syntax_tree) = editor.buffer_manager.active().syntax_tree.as_ref() {
                            let byte = self.buffer.offset_for_anchor(&cursor.head());
                            if let Some((start_node, end_node)) = syntax_tree.delimiter_boundaries_at_byte(byte) {
                                let matches_ch = match ch {
                                    '{' | '}' => start_node.kind == "{",
                                    '(' | ')' => start_node.kind == "(",
                                    '[' | ']' => start_node.kind == "[",
                                    '"' => start_node.kind == "\"",
                                    '\'' => start_node.kind == "'",
                                    '`' => start_node.kind == "`",
                                    't' | '<' | '>' => start_node.kind == "<" || start_node.kind == "start_tag" || start_node.kind == "jsx_opening_element",
                                    _ => false,
                                };
                                if matches_ch {
                                    let start_offset = start_node.byte_range.end;
                                    let end_offset = end_node.byte_range.start.saturating_sub(1);
                                    let start_anchor = self.buffer.anchor_at(start_offset, Bias::Left);
                                    let end_anchor = self.buffer.anchor_at(end_offset, Bias::Right);
                                    let next = Selection {
                                        id: cursor.id,
                                        start: start_anchor,
                                        end: end_anchor,
                                        reversed: false,
                                        goal: SelectionGoal::None,
                                    };
                                    self.selections.update(&self.buffer, &next);
                                    updated = true;
                                }
                            }
                        }
                    }
                    if !updated {
                        let next = cursor.move_within_character(select, *count, *ch, &self.buffer);
                        self.selections.update(&self.buffer, &next);
                    }
                }
            }
            Action::MoveAroundCharacter { count, ch } => {
                let select = self.current_mode().is_visual();
                let cursors = self.selections.selections.clone();
                for cursor in cursors.iter() {
                    let mut updated = false;
                    if *ch == 'w' {
                        let start_sel = cursor.move_to_word(false, &self.buffer);
                        let next_word_head = cursor.move_to_next_word(false, &self.buffer).head();
                        let next_word_offset = self.buffer.offset_for_anchor(&next_word_head);
                        let end_offset = self.buffer.clip_offset(next_word_offset.saturating_sub(1), Bias::Left);
                        let next = Selection {
                            id: cursor.id,
                            start: start_sel.head(),
                            end: self.buffer.anchor_at(end_offset, Bias::Right),
                            reversed: false,
                            goal: SelectionGoal::None,
                        };
                        self.selections.update(&self.buffer, &next);
                        updated = true;
                    } else if *ch == 'p' {
                        let prev_p = cursor.move_to_previous_paragraph(false, &self.buffer).head().to_point(&self.buffer);
                        let next_p = cursor.move_to_next_paragraph(false, &self.buffer).head().to_point(&self.buffer);
                        let start_row = if prev_p.row < self.buffer.row_count() && self.buffer.line_len(prev_p.row) == 0 {
                            prev_p.row + 1
                        } else {
                            prev_p.row
                        };
                        let end_row = next_p.row;
                        let start_offset = Point { row: start_row, column: 0 }.to_offset(&self.buffer);
                        let end_offset = Point { row: end_row, column: self.buffer.line_len(end_row) }.to_offset(&self.buffer);
                        let next = Selection {
                            id: cursor.id,
                            start: self.buffer.anchor_at(start_offset, Bias::Left),
                            end: self.buffer.anchor_at(end_offset, Bias::Right),
                            reversed: false,
                            goal: SelectionGoal::None,
                        };
                        self.selections.update(&self.buffer, &next);
                        updated = true;
                    } else if editor.tree_sitter {
                        if let Some(syntax_tree) = editor.buffer_manager.active().syntax_tree.as_ref() {
                            let byte = self.buffer.offset_for_anchor(&cursor.head());
                            if let Some((start_node, end_node)) = syntax_tree.delimiter_boundaries_at_byte(byte) {
                                let matches_ch = match ch {
                                    '{' | '}' => start_node.kind == "{",
                                    '(' | ')' => start_node.kind == "(",
                                    '[' | ']' => start_node.kind == "[",
                                    '"' => start_node.kind == "\"",
                                    '\'' => start_node.kind == "'",
                                    '`' => start_node.kind == "`",
                                    't' | '<' | '>' => start_node.kind == "<" || start_node.kind == "start_tag" || start_node.kind == "jsx_opening_element",
                                    _ => false,
                                };
                                if matches_ch {
                                    let start_offset = start_node.byte_range.start;
                                    let end_offset = end_node.byte_range.end.saturating_sub(1);
                                    let start_anchor = self.buffer.anchor_at(start_offset, Bias::Left);
                                    let end_anchor = self.buffer.anchor_at(end_offset, Bias::Right);
                                    let next = Selection {
                                        id: cursor.id,
                                        start: start_anchor,
                                        end: end_anchor,
                                        reversed: false,
                                        goal: SelectionGoal::None,
                                    };
                                    self.selections.update(&self.buffer, &next);
                                    updated = true;
                                }
                            }
                        }
                    }
                    if !updated {
                        let next = cursor.move_around_character(select, *count, *ch, &self.buffer);
                        self.selections.update(&self.buffer, &next);
                    }
                }
            }

            Action::MoveToNextFunction { select, count } => {
                if editor.tree_sitter && *count > 0 {
                    if let Some(syntax_tree) = editor.buffer_manager.active().syntax_tree.as_ref() {
                        self.selections.move_to_syntax_target(
                            *select,
                            *count,
                            syntax_tree,
                            &self.buffer,
                            |tree, byte| tree.next_function_after_byte(byte),
                        );
                    }
                }
            }
            Action::MoveToPreviousFunction { select, count } => {
                if editor.tree_sitter && *count > 0 {
                    if let Some(syntax_tree) = editor.buffer_manager.active().syntax_tree.as_ref() {
                        self.selections.move_to_syntax_target(
                            *select,
                            *count,
                            syntax_tree,
                            &self.buffer,
                            |tree, byte| tree.previous_function_before_byte(byte),
                        );
                    }
                }
            }
            Action::MoveToNextBlock { select, count } => {
                if editor.tree_sitter && *count > 0 {
                    if let Some(syntax_tree) = editor.buffer_manager.active().syntax_tree.as_ref() {
                        self.selections.move_to_syntax_target(
                            *select,
                            *count,
                            syntax_tree,
                            &self.buffer,
                            |tree, byte| tree.next_block_after_byte(byte),
                        );
                    }
                }
            }
            Action::MoveToPreviousBlock { select, count } => {
                if editor.tree_sitter && *count > 0 {
                    if let Some(syntax_tree) = editor.buffer_manager.active().syntax_tree.as_ref() {
                        self.selections.move_to_syntax_target(
                            *select,
                            *count,
                            syntax_tree,
                            &self.buffer,
                            |tree, byte| tree.previous_block_before_byte(byte),
                        );
                    }
                }
            }
            Action::MoveToBlockStart { select, count } => {
                if editor.tree_sitter && *count > 0 {
                    if let Some(syntax_tree) = editor.buffer_manager.active().syntax_tree.as_ref() {
                        self.selections.move_to_syntax_target(
                            *select,
                            *count,
                            syntax_tree,
                            &self.buffer,
                            |tree, byte| tree.block_start_at_byte(byte),
                        );
                    }
                }
            }
            Action::MoveToBlockEnd { select, count } => {
                if editor.tree_sitter && *count > 0 {
                    if let Some(syntax_tree) = editor.buffer_manager.active().syntax_tree.as_ref() {
                        self.selections.move_to_syntax_target_end(
                            *select,
                            *count,
                            syntax_tree,
                            &self.buffer,
                            |tree, byte| tree.block_end_at_byte(byte),
                        );
                    }
                }
            }
            Action::MoveToNextClass { select, count } => {
                if editor.tree_sitter && *count > 0 {
                    if let Some(syntax_tree) = editor.buffer_manager.active().syntax_tree.as_ref() {
                        self.selections.move_to_syntax_target(
                            *select,
                            *count,
                            syntax_tree,
                            &self.buffer,
                            |tree, byte| tree.next_class_after_byte(byte),
                        );
                    }
                }
            }
            Action::MoveToPreviousClass { select, count } => {
                if editor.tree_sitter && *count > 0 {
                    if let Some(syntax_tree) = editor.buffer_manager.active().syntax_tree.as_ref() {
                        self.selections.move_to_syntax_target(
                            *select,
                            *count,
                            syntax_tree,
                            &self.buffer,
                            |tree, byte| tree.previous_class_before_byte(byte),
                        );
                    }
                }
            }
            Action::MoveToNextArgument { select, count } => {
                if editor.tree_sitter && *count > 0 {
                    if let Some(syntax_tree) = editor.buffer_manager.active().syntax_tree.as_ref() {
                        self.selections.move_to_syntax_target(
                            *select,
                            *count,
                            syntax_tree,
                            &self.buffer,
                            |tree, byte| tree.next_argument_after_byte(byte),
                        );
                    }
                }
            }
            Action::MoveToPreviousArgument { select, count } => {
                if editor.tree_sitter && *count > 0 {
                    if let Some(syntax_tree) = editor.buffer_manager.active().syntax_tree.as_ref() {
                        self.selections.move_to_syntax_target(
                            *select,
                            *count,
                            syntax_tree,
                            &self.buffer,
                            |tree, byte| tree.previous_argument_before_byte(byte),
                        );
                    }
                }
            }

            Action::MoveToStartOfDocument { select, count } => self
                .selections
                .move_to_start_of_document(*select, &self.buffer),
            Action::MoveToEndOfDocument { select, count } => self
                .selections
                .move_to_end_of_document(*select, &self.buffer),
            Action::MoveToStartOfLine { select, count } => {
                self.selections.move_to_start_of_line(*select, &self.buffer)
            }
            Action::MoveToStartOfLineNonSpace { select, count } => self
                .selections
                .move_to_start_of_line_non_space(*select, &self.buffer),
            Action::MoveToEndOfLine { select, count } => {
                self.selections.move_to_end_of_line(*select, &self.buffer)
            }
            Action::MoveToStartOfPreviousLine { select, count } => self
                .selections
                .move_to_start_of_previous_line(*select, &self.buffer),
            Action::MoveToEndOfPreviousLine { select, count } => self
                .selections
                .move_to_end_of_previous_line(*select, &self.buffer),
            Action::MoveToStartOfNextLine { select, count } => self
                .selections
                .move_to_start_of_next_line(*select, &self.buffer),
            Action::MoveToEndOfNextLine { select, count } => self
                .selections
                .move_to_end_of_next_line(*select, &self.buffer),
            Action::MovePageUp { count, select } => {
                let page_size = (editor.screen_rows - 4).max(1) as u32;
                self.selections.move_up(*select, page_size * *count, &self.buffer)
            }
            Action::MovePageDown { count, select } => {
                let page_size = (editor.screen_rows - 4).max(1) as u32;
                self.selections.move_down(*select, page_size * *count, &self.buffer)
            }
            Action::ScrollHalfPageUp { count } => {
                let half_page_size = ((editor.screen_rows - 4).max(1) / 2).max(1) as u32;
                self.selections.move_up(false, half_page_size * *count, &self.buffer)
            }
            Action::ScrollHalfPageDown { count } => {
                let half_page_size = ((editor.screen_rows - 4).max(1) / 2).max(1) as u32;
                self.selections.move_down(false, half_page_size * *count, &self.buffer)
            }
            Action::MoveToScreenTop { select, count } => {
                let active_idx = editor.buffer_manager.active_idx;
                let buffer = &editor.buffer_manager.buffers[active_idx];
                let display_snapshot = buffer.display_map.snapshot();
                let target_point = display_snapshot.display_point_to_point(
                    crate::display::display_map::DisplayPoint::new(display_snapshot.scroll_y, 0)
                );
                self.selections.move_to_line(*select, target_point.row, &self.buffer)
            }
            Action::MoveToScreenMiddle { select, count } => {
                let active_idx = editor.buffer_manager.active_idx;
                let buffer = &editor.buffer_manager.buffers[active_idx];
                let display_snapshot = buffer.display_map.snapshot();
                let middle_display_row = display_snapshot.scroll_y + display_snapshot.visible_rows / 2;
                let target_point = display_snapshot.display_point_to_point(
                    crate::display::display_map::DisplayPoint::new(middle_display_row, 0)
                );
                self.selections.move_to_line(*select, target_point.row, &self.buffer)
            }
            Action::MoveToScreenBottom { select, count } => {
                let active_idx = editor.buffer_manager.active_idx;
                let buffer = &editor.buffer_manager.buffers[active_idx];
                let display_snapshot = buffer.display_map.snapshot();
                let bottom_display_row = display_snapshot.scroll_y + display_snapshot.visible_rows.saturating_sub(1);
                let target_point = display_snapshot.display_point_to_point(
                    crate::display::display_map::DisplayPoint::new(bottom_display_row, 0)
                );
                self.selections.move_to_line(*select, target_point.row, &self.buffer)
            }
            Action::InsertText(text) => {
                self.delete_text(0);
                self.insert_text(text);
            }
            Action::DeleteChar { count } | Action::Delete { count } => {
                let text = if self.selections.has_selection(&self.buffer) {
                    self.selections.text(&self.buffer)
                } else {
                    let head_offset = self.buffer.offset_for_anchor(&self.selection().head());
                    let end_offset = self
                        .buffer
                        .clip_offset(head_offset + *count as usize, Bias::Right);
                    self.buffer
                        .as_rope()
                        .chunks_in_range(head_offset..end_offset)
                        .collect()
                };
                editor.clipboard.borrow_mut().set_text(&text);

                if self.delete_text(0) {
                    //
                } else {
                    for _ in 0..*count {
                        self.delete_text(1);
                    }
                }
            }
            Action::DeleteCharBefore { count } => {
                let text = if self.selections.has_selection(&self.buffer) {
                    self.selections.text(&self.buffer)
                } else {
                    let head_offset = self.buffer.offset_for_anchor(&self.selection().head());
                    let start_offset = if head_offset >= *count as usize {
                        head_offset - *count as usize
                    } else {
                        0
                    };
                    self.buffer
                        .as_rope()
                        .chunks_in_range(start_offset..head_offset)
                        .collect()
                };
                editor.clipboard.borrow_mut().set_text(&text);

                if self.delete_text(0) {
                    //
                } else {
                    for _ in 0..*count {
                        self.selections.move_left(false, 1, &self.buffer);
                        self.delete_text(1);
                    }
                }
            }
            Action::DeleteLines {
                start_line,
                end_line,
            } => {
                let start_row = start_line.saturating_sub(1);
                let end_row = end_line.saturating_sub(1);

                let max_row = self.buffer.row_count().saturating_sub(1);
                let start_row = std::cmp::min(start_row, max_row);
                let end_row = std::cmp::min(end_row, max_row);
                let start_row = std::cmp::min(start_row, end_row);

                let start_offset = Point::new(start_row, 0).to_offset(&self.buffer);
                let end_offset = if end_row + 1 < self.buffer.row_count() {
                    Point::new(end_row + 1, 0).to_offset(&self.buffer)
                } else {
                    Point::new(end_row, self.buffer.line_len(end_row)).to_offset(&self.buffer)
                };

                let text: String = self
                    .buffer
                    .as_rope()
                    .chunks_in_range(start_offset..end_offset)
                    .collect();
                editor.clipboard.borrow_mut().set_lines(text);

                self.buffer.edit([(start_offset..end_offset, "")]);
            }
            Action::YankLines {
                start_line,
                end_line,
            } => {
                let start_row = start_line.saturating_sub(1);
                let end_row = end_line.saturating_sub(1);

                let max_row = self.buffer.row_count().saturating_sub(1);
                let start_row = std::cmp::min(start_row, max_row);
                let end_row = std::cmp::min(end_row, max_row);
                let start_row = std::cmp::min(start_row, end_row);

                let start_offset = Point::new(start_row, 0).to_offset(&self.buffer);
                let end_offset = if end_row + 1 < self.buffer.row_count() {
                    Point::new(end_row + 1, 0).to_offset(&self.buffer)
                } else {
                    Point::new(end_row, self.buffer.line_len(end_row)).to_offset(&self.buffer)
                };

                let text: String = self
                    .buffer
                    .as_rope()
                    .chunks_in_range(start_offset..end_offset)
                    .collect();
                editor.clipboard.borrow_mut().set_lines(text);
            }
            Action::DeleteLine { count } | Action::ChangeLine { count } => {
                let selections = self.selections.selections.clone();
                let point = self.selections.point;
                let anchor = self.selections.anchor.clone();

                self.selections.move_to_start_of_line(false, &self.buffer);
                if *count > 1 {
                    self.selections
                        .move_down(true, count.saturating_sub(1), &self.buffer);
                }
                self.selections.move_to_end_of_line(true, &self.buffer);

                let mut text = self.selections.text(&self.buffer);
                if !text.ends_with('\n') {
                    text.push('\n');
                }
                editor.clipboard.borrow_mut().set_lines(text);

                self.selections.selections = selections;
                self.selections.point = point;
                self.selections.anchor = anchor;

                self.delete_current_line(*count);
            }
            Action::JoinLines { count } => {
                let count = *count;
                let lines_to_join = if count <= 1 { 2 } else { count };
                let newlines_to_remove = lines_to_join - 1;

                let current_row = self
                    .selections
                    .first()
                    .unwrap()
                    .head()
                    .to_point(&self.buffer)
                    .row;
                let total_rows = self.buffer.row_count();
                let actual_removes = std::cmp::min(
                    newlines_to_remove as usize,
                    (total_rows.saturating_sub(1) - current_row) as usize,
                );

                let mut target_col = None;

                for _ in 0..actual_removes {
                    let current_line_len = self.buffer.line_len(current_row);
                    if target_col.is_none() {
                        target_col = Some(current_line_len);
                    }

                    let end_of_current = Point {
                        row: current_row,
                        column: current_line_len,
                    }
                    .to_offset(&self.buffer);
                    let next_line_text = self.buffer.row_text(current_row + 1);
                    let leading_whitespace_len = next_line_text
                        .chars()
                        .take_while(|c| c.is_whitespace())
                        .map(|c| c.len_utf8())
                        .sum::<usize>();

                    let delete_start = end_of_current;
                    let delete_end = end_of_current + 1 + leading_whitespace_len;

                    let current_line_text = self.buffer.row_text(current_row);
                    let ends_with_space = current_line_text.as_str().ends_with(char::is_whitespace);
                    let next_first_non_space = next_line_text.as_str().trim_start().chars().next();

                    let replacement = if ends_with_space || next_first_non_space.is_none() {
                        ""
                    } else {
                        " "
                    };

                    self.buffer.edit([(delete_start..delete_end, replacement)]);
                }

                if let Some(col) = target_col {
                    let target_point = Point {
                        row: current_row,
                        column: col,
                    };
                    let target_anchor = self
                        .buffer
                        .anchor_at(target_point.to_offset(&self.buffer), Bias::Left);
                    self.selections.clear(&self.buffer);
                    let first = self.selections.first().unwrap().clone();
                    let next = Selection {
                        id: first.id,
                        start: target_anchor.clone(),
                        end: target_anchor,
                        reversed: false,
                        goal: SelectionGoal::None,
                    };
                    self.selections.point = target_point;
                    self.selections.update(&self.buffer, &next);
                }
            }
            Action::ChangeMotion { count, motion } | Action::DeleteMotion { count, motion } => {
                let mut motion = (**motion).clone();
                let is_textobject = match &motion {
                    Action::MoveToWord { .. }
                    | Action::MoveToNextParagraph { .. }
                    | Action::MoveToEndOfLine { .. }
                    | Action::MoveWithinCharacter { .. }
                    | Action::MoveAroundCharacter { .. } => true,
                    _ => false,
                };

                match &mut motion {
                    Action::MoveUp { select, .. }
                    | Action::MoveDown { select, .. }
                    | Action::MoveLeft { select, .. }
                    | Action::MoveRight { select, .. }
                    | Action::MoveToPreviousWord { select, .. }
                    | Action::MoveToWord { select, .. }
                    | Action::MoveToPreviousWordEnd { select, .. }
                    | Action::MoveToWordEnd { select, .. }
                    | Action::MoveToStartOfDocument { select, .. }
                    | Action::MoveToEndOfDocument { select, .. }
                    | Action::MoveToStartOfLine { select, .. }
                    | Action::MoveToStartOfLineNonSpace { select, .. }
                    | Action::MoveToEndOfLine { select, .. }
                    | Action::MoveToPreviousParagraph { select, .. }
                    | Action::MoveToNextParagraph { select, .. }
                    | Action::MoveToPreviousCharacter { select, .. }
                    | Action::MoveToNextCharacter { select, .. } => *select = true,
                    _ => {}
                }

                let selections = self.selections.selections.clone();
                let point = self.selections.point;
                let anchor = self.selections.anchor.clone();

                for _ in 0..*count {
                    self.apply_action(&motion, editor);
                }

                let text = self.selections.text(&self.buffer);
                editor.clipboard.borrow_mut().set_text(text);

                self.selections.selections = selections;
                self.selections.point = point;
                self.selections.anchor = anchor;

                if is_textobject {
                    let inclusive = matches!(motion, Action::MoveWithinCharacter { .. } | Action::MoveAroundCharacter { .. });
                    for _idx in 0..*count {
                        self.apply_action(&motion, editor);
                        self.delete_text_object(inclusive);
                    }
                } else {
                    for _ in 0..*count {
                        self.apply_action(&motion, editor);
                        self.delete_text(0);
                    }
                }
            }
            Action::Change { count } => {
                let text = self.selections.text(&self.buffer);
                if !text.is_empty() {
                    editor.clipboard.borrow_mut().set_text(text);
                }
                self.delete_text(0);
            }
            Action::InsertNewLine { count } => {
                let text = self.selections.text(&self.buffer);
                if !text.is_empty() {
                    editor.clipboard.borrow_mut().set_text(text);
                }
                self.delete_text(0);
                for _ in 0..*count {
                    self.insert_text(&self.new_line().to_string());
                }
            }
            Action::InsertNewLineMotion { count, motion } => {
                let mut motion = (**motion).clone();
                for _ in 0..*count {
                    self.apply_action(&motion, editor);
                    self.insert_text(&self.new_line().to_string());
                    motion = Action::NoOp;
                }
                self.selections.move_left(false, 1, &self.buffer);
            }
            Action::InsertTab => {
                for _ in 0..4 {
                    self.insert_text(" ");
                }
            }
            Action::YankMotion { count, motion } => {
                self.yank_motion(*count, motion, editor);
            }
            Action::YankLine { count } => {
                self.yank_current_line(*count, editor);
            }
            Action::Put { count } => {
                self.paste(*count, editor);
            }
             Action::Undo { count } => self.undo(*count),
            Action::Redo { count } => self.redo(*count),
            Action::Fold { count } => {
                self.fold(*count, editor);
            }
            Action::Unfold { count } => {
                self.unfold(*count, editor);
            }
            Action::NoOp | Action::Quit => {
                return;
            }
            _ => {}
        }

        self.apply_action(&next_action, editor);
        self.snap_selections_to_folds(action);
    }

    fn yank_motion(&mut self, count: u32, motion: &Action, editor: &Editor) {
        let selections = self.selections.selections.clone();
        let point = self.selections.point;
        let anchor = self.selections.anchor.clone();

        for _ in 0..count {
            self.apply_action(motion, editor);
        }
        let text = self.selections.text(&self.buffer);
        editor.clipboard.borrow_mut().set_text(text);

        self.selections.selections = selections;
        self.selections.point = point;
        self.selections.anchor = anchor;
    }

    fn yank_current_line(&mut self, count: u32, editor: &Editor) {
        let selections = self.selections.selections.clone();
        let point = self.selections.point;
        let anchor = self.selections.anchor.clone();

        self.selections.move_to_start_of_line(false, &self.buffer);
        if count > 1 {
            self.selections
                .move_down(true, count.saturating_sub(1), &self.buffer);
        }
        self.selections.move_to_end_of_line(true, &self.buffer);

        let mut text = self.selections.text(&self.buffer);
        if !text.ends_with('\n') {
            text.push('\n');
        }
        editor.clipboard.borrow_mut().set_lines(text);

        self.selections.selections = selections;
        self.selections.point = point;
        self.selections.anchor = anchor;
    }

    fn paste(&mut self, count: u32, editor: &Editor) {
        let clipboard = editor.clipboard.borrow();
        if clipboard.is_empty() || count == 0 {
            return;
        }
        let text = clipboard.text().to_string();
        let kind = clipboard.kind();
        drop(clipboard);

        match kind {
            ClipboardKind::Character | ClipboardKind::Block => {
                self.selections.move_right(false, 1, &self.buffer);
                for _ in 0..count {
                    self.insert_text(&text);
                }
            }
            ClipboardKind::Line => {
                let cursor_row = self.selection().head().to_point(&self.buffer).row;
                let has_next_line = cursor_row + 1 < self.buffer.row_count();
                if has_next_line {
                    self.selections
                        .move_to_start_of_next_line(false, &self.buffer);
                } else {
                    self.selections.move_to_end_of_line(false, &self.buffer);
                    self.insert_text(&self.new_line().to_string());
                }
                for _ in 0..count {
                    self.insert_text(&text);
                }
            }
        }
    }

    fn insert_text(&mut self, text: &str) {
        let cursors = self.selections.selections.clone();
        for cursor in cursors.iter() {
            let start = self.buffer.offset_for_anchor(&cursor.head());
            self.buffer.edit([(start..start, text)]);

            let new_offset = self.buffer.clip_offset(start + text.len(), Bias::Left);
            let new_head = self.buffer.anchor_at(new_offset, Bias::Left);
            self.selections.update(
                &self.buffer,
                &Selection {
                    id: cursor.id,
                    start: new_head,
                    end: new_head,
                    reversed: false,
                    goal: SelectionGoal::None,
                },
            );
        }
    }

    fn delete_text(&mut self, count: usize) -> bool {
        let mut delete_count = 0;
        let cursors = self.selections.selections.clone();
        for cursor in cursors.iter() {
            let (start, mut end) = {
                let (cs, ce) = if cursor.head().cmp(&cursor.tail(), &self.buffer) == Ordering::Less
                {
                    (
                        cursor.head().bias_left(&self.buffer),
                        cursor.tail().bias_right(&self.buffer),
                    )
                } else {
                    (
                        cursor.tail().bias_left(&self.buffer),
                        cursor.head().bias_right(&self.buffer),
                    )
                };

                let start = self.buffer.offset_for_anchor(&cs);
                let mut end = self.buffer.offset_for_anchor(&ce);
                if start != end {
                    end = self.buffer.clip_offset(end + 1, Bias::Right);
                }
                (start, end)
            };

            if count != 0 {
                end = self.buffer.clip_offset(end + count, Bias::Right);
            }

            if start != end {
                delete_count += 1;
                self.remove_overlapping_folds(start, end);
                self.buffer.edit([(start..end, "")]);
            }
        }
        return delete_count > 0;
    }

    fn delete_text_object(&mut self, inclusive: bool) -> bool {
        let mut delete_count = 0;
        let cursors = self.selections.selections.clone();
        for cursor in cursors.iter() {
            let (start, mut end) = {
                let (cs, ce) = if cursor.head().cmp(&cursor.tail(), &self.buffer) == Ordering::Less
                {
                    (
                        cursor.head().bias_left(&self.buffer),
                        cursor.tail().bias_right(&self.buffer),
                    )
                } else {
                    (
                        cursor.tail().bias_left(&self.buffer),
                        cursor.head().bias_right(&self.buffer),
                    )
                };

                let start = self.buffer.offset_for_anchor(&cs);
                let mut end = self.buffer.offset_for_anchor(&ce);
                if inclusive && start != end {
                    end = self.buffer.clip_offset(end + 1, Bias::Right);
                }
                (start, end)
            };

            if start != end {
                delete_count += 1;
                self.buffer.edit([(start..end, "")]);
            }
        }
        return delete_count > 0;
    }

    pub fn delete_current_line(&mut self, count: u32) {
        if self.delete_text(0) {
            return;
        }
        for _ in 0..count {
            let cursors = self.selections.selections.clone();
            for cursor in cursors.iter() {
                let mut point = cursor.head().to_point(&self.buffer);
                let (start, end) = {
                    point.column = 0;
                    let start = self
                        .buffer
                        .offset_for_anchor(&self.buffer.anchor_at(&point, Bias::Left));
                    if point.row < self.buffer.row_count() {
                        point.row += 1;
                    } else {
                        point.column = self.buffer.line_len(point.row);
                    }
                    let end = self.buffer.clip_offset(
                        self.buffer
                            .offset_for_anchor(&self.buffer.anchor_at(&point, Bias::Right)),
                        Bias::Right,
                    );
                    (start, end)
                };
                if start != end {
                    self.remove_overlapping_folds(start, end);
                    self.buffer.edit([(start..end, "")]);
                }
            }
        }
    }

    fn remove_overlapping_folds(&mut self, start: usize, end: usize) {
        let start_point = start.to_point(&self.buffer);
        let end_point = end.to_point(&self.buffer);
        self.folds.retain(|fold| {
            !(fold.end > start_point && fold.start < end_point)
        });
    }

    pub fn selection(&self) -> Selection<Anchor> {
        self.selections.first().unwrap().clone()
    }

    pub fn add_selection(&mut self) -> Selection<Anchor> {
        self.selections.add(&self.buffer, 0)
    }

    pub fn selections(&self) -> &SelectionCollection {
        &self.selections
    }

    pub fn clear_selections(&mut self) {
        self.selections.clear(&self.buffer);
    }

    pub fn has_selection(&self) -> bool {
        self.selections.has_selection(&self.buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::treesitter::TreeSitterParser;
    use crate::treesitter::grammars::Grammar;

    #[test]
    fn consecutive_insert_text_actions_leave_cursor_after_inserted_text() {
        let mut editor = Editor::new(Vec::new()).unwrap();
        editor
            .buffer_manager
            .active_mut()
            .doc
            .enter_mode(Mode::Insert);
        editor.apply_active_action(&Action::InsertText("abc".into()));
        editor.apply_active_action(&Action::MoveLeft {
            select: false,
            count: 2,
        });
        editor.apply_active_action(&Action::InsertText("x".into()));
        editor.apply_active_action(&Action::InsertText("y".into()));

        let document = &editor.buffer_manager.active().doc;
        assert_eq!(document.buffer().row_text(0), "axybc");
        assert_eq!(
            document
                .selection()
                .head()
                .to_point(document.buffer())
                .column,
            3
        );
    }

    #[test]
    fn newline_and_tab_insertions_do_not_advance_twice() {
        let mut newline_editor = Editor::new(Vec::new()).unwrap();
        newline_editor
            .buffer_manager
            .active_mut()
            .doc
            .enter_mode(Mode::Insert);
        newline_editor.apply_active_action(&Action::InsertText("abc".into()));
        newline_editor.apply_active_action(&Action::MoveLeft {
            select: false,
            count: 2,
        });
        newline_editor.apply_active_action(&Action::InsertNewLine { count: 1 });

        let newline_document = &newline_editor.buffer_manager.active().doc;
        assert_eq!(newline_document.buffer().row_text(0), "a");
        assert_eq!(newline_document.buffer().row_text(1), "bc");
        assert_eq!(
            newline_document
                .selection()
                .head()
                .to_point(newline_document.buffer()),
            Point::new(1, 0)
        );

        let mut tab_editor = Editor::new(Vec::new()).unwrap();
        tab_editor
            .buffer_manager
            .active_mut()
            .doc
            .enter_mode(Mode::Insert);
        tab_editor.apply_active_action(&Action::InsertText("abc".into()));
        tab_editor.apply_active_action(&Action::MoveLeft {
            select: false,
            count: 2,
        });
        tab_editor.apply_active_action(&Action::InsertTab);

        let tab_document = &tab_editor.buffer_manager.active().doc;
        assert_eq!(tab_document.buffer().row_text(0), "a    bc");
        assert_eq!(
            tab_document
                .selection()
                .head()
                .to_point(tab_document.buffer())
                .column,
            5
        );
    }

    // #[test]
    // fn tree_sitter_actions_navigate_functions_classes_and_arguments() {
    //     let source = "\nstruct Alpha {}\nfn first(a: i32, b: i32) {}\nfn second(c: i32) {}";
    //     let mut editor = Editor::new(Vec::new()).unwrap();
    //     editor.apply_active_action(&Action::InsertText(source.into()));

    //     let syntax_tree = {
    //         let document = &editor.buffer_manager.active().doc;
    //         let mut parser = TreeSitterParser::new(Grammar::Rust).unwrap();
    //         parser.parse(document.buffer().snapshot(), None).unwrap()
    //     };
    //     editor.buffer_manager.active_mut().syntax_tree = Some(syntax_tree);

    //     editor.apply_active_action(&Action::MoveToStartOfDocument { select: false, count: 1 });
    //     editor.apply_active_action(&Action::MoveToNextWord {
    //         select: false,
    //         count: 1,
    //     });
    //     assert_eq!(
    //         editor
    //             .buffer_manager
    //             .active()
    //             .doc
    //             .selection()
    //             .head()
    //             .to_point(editor.buffer_manager.active().doc.buffer()),
    //         Point::new(1, 0)
    //     );

    //     editor.apply_active_action(&Action::MoveToStartOfDocument { select: false, count: 1 });
    //     editor.apply_active_action(&Action::MoveToNextFunction {
    //         select: false,
    //         count: 2,
    //     });
    //     assert_eq!(
    //         editor
    //             .buffer_manager
    //             .active()
    //             .doc
    //             .selection()
    //             .head()
    //             .to_point(editor.buffer_manager.active().doc.buffer()),
    //         Point::new(3, 0)
    //     );
    //     editor.apply_active_action(&Action::MoveToPreviousFunction {
    //         select: false,
    //         count: 1,
    //     });
    //     assert_eq!(
    //         editor
    //             .buffer_manager
    //             .active()
    //             .doc
    //             .selection()
    //             .head()
    //             .to_point(editor.buffer_manager.active().doc.buffer()),
    //         Point::new(2, 0)
    //     );

    //     editor.apply_active_action(&Action::MoveToStartOfDocument { select: false, count: 1 });
    //     editor.apply_active_action(&Action::MoveToNextArgument {
    //         select: false,
    //         count: 2,
    //     });
    //     let document = &editor.buffer_manager.active().doc;
    //     let offset = document
    //         .buffer()
    //         .offset_for_anchor(&document.selection().head());
    //     assert_eq!(&source[offset..offset + 1], "b");

    //     editor.apply_active_action(&Action::MoveToPreviousArgument {
    //         select: false,
    //         count: 1,
    //     });
    //     let document = &editor.buffer_manager.active().doc;
    //     let offset = document
    //         .buffer()
    //         .offset_for_anchor(&document.selection().head());
    //     assert_eq!(&source[offset..offset + 1], "a");
    // }

    // Test movement with selection
    // #[test]
    // fn tree_sitter_motions() {
    //     let mut editor = Editor::new(Vec::new()).unwrap();
    //     editor.set_tree_sitter_enabled(true);
    //     editor.apply_active_action(&Action::InsertText(
    //         "\nstruct Alpha {}\nfn first(a: i32, b: i32) {}\nfn second(c: i32) {}".into(),
    //     ));
    //     editor.apply_active_action(&Action::MoveToStartOfDocument { select: false, count: 1 });
    //     editor.apply_active_action(&Action::MoveToNextClass {
    //         select: false,
    //         count: 1,
    //     });
    //     let document = &editor.buffer_manager.active().doc;
    //     assert_eq!(
    //         document.selection().head().to_point(document.buffer()),
    //         Point::new(1, 0)
    //     );

    //     editor.apply_active_action(&Action::MoveToStartOfDocument { select: false, count: 1 });
    //     editor.apply_active_action(&Action::MoveToNextFunction {
    //         select: false,
    //         count: 1,
    //     });
    //     let document = &editor.buffer_manager.active().doc;
    //     assert_eq!(
    //         document.selection().head().to_point(document.buffer()),
    //         Point::new(2, 0)
    //     );

    //     editor.apply_active_action(&Action::MoveToPreviousFunction {
    //         select: false,
    //         count: 1,
    //     });
    //     let document = &editor.buffer_manager.active().doc;
    //     assert_eq!(
    //         document.selection().head().to_point(document.buffer()),
    //         Point::new(2, 0)
    //     );

    //     editor.apply_active_action(&Action::MoveToStartOfDocument { select: false, count: 1 });
    //     editor.apply_active_action(&Action::MoveToNextArgument {
    //         select: false,
    //         count: 1,
    //     });
    //     let document = &editor.buffer_manager.active().doc;
    //     assert_eq!(
    //         document.selection().head().to_point(document.buffer()),
    //         Point::new(2, 9)
    //     );

    //     editor.apply_active_action(&Action::MoveToNextArgument {
    //         select: false,
    //         count: 1,
    //     });
    //     let document = &editor.buffer_manager.active().doc;
    //     assert_eq!(
    //         document.selection().head().to_point(document.buffer()),
    //         Point::new(2, 17)
    //     );

    //     editor.apply_active_action(&Action::MoveToNextFunction {
    //         select: false,
    //         count: 1,
    //     }); // move to fn first
    //     editor.apply_active_action(&Action::MoveToNextBlock {
    //         select: false,
    //         count: 1,
    //     }); // move to {
    //     let document = &editor.buffer_manager.active().doc;
    //     assert_eq!(
    //         document.selection().head().to_point(document.buffer()),
    //         Point::new(2, 25)
    //     );

    //     editor.apply_active_action(&Action::MoveToBlockStart {
    //         select: false,
    //         count: 1,
    //     });
    //     let document = &editor.buffer_manager.active().doc;
    //     // In the source: "\nstruct Alpha {}\nfn first(a: i32, b: i32) {}\nfn second(c: i32) {}"
    //     // fn first is at row 2. { is at (2, 25).
    //     assert_eq!(
    //         document.selection().head().to_point(document.buffer()),
    //         Point::new(2, 25)
    //     );

    //     editor.apply_active_action(&Action::MoveToBlockEnd {
    //         select: false,
    //         count: 1,
    //     });
    //     let document = &editor.buffer_manager.active().doc;
    //     // end of {} block is at (2, 26)
    //     assert_eq!(
    //         document.selection().head().to_point(document.buffer()),
    //         Point::new(2, 26)
    //     );
    // }

    #[test]
    fn yank_motion_copies_selection_and_paste_inserts_after_cursor() {
        let mut editor = Editor::new(Vec::new()).unwrap();
        editor.apply_active_action(&Action::InsertText("abcde".into()));
        editor.apply_active_action(&Action::MoveLeft {
            select: false,
            count: 4,
        });

        editor.apply_active_action(&Action::YankMotion {
            count: 1,
            motion: Box::new(Action::MoveRight {
                select: true,
                count: 1,
            }),
        });

        assert_eq!(editor.clipboard.borrow().text(), "bc");
        assert_eq!(
            editor
                .buffer_manager
                .active()
                .doc
                .selection()
                .head()
                .to_point(editor.buffer_manager.active().doc.buffer())
                .column,
            1
        );

        editor.apply_active_action(&Action::Put { count: 1 });

        assert_eq!(
            editor.buffer_manager.active().doc.buffer().row_text(0),
            "abbccde"
        );
    }

    #[test]
    fn yank_current_line_and_paste_create_a_line_below() {
        let mut editor = Editor::new(Vec::new()).unwrap();
        editor.apply_active_action(&Action::InsertText("abc".into()));
        editor.apply_active_action(&Action::MoveLeft {
            select: false,
            count: 1,
        });

        editor.apply_active_action(&Action::YankLine { count: 1 });
        assert_eq!(editor.clipboard.borrow().text(), "abc\n");
        assert_eq!(editor.clipboard.borrow().kind(), ClipboardKind::Line);

        editor.apply_active_action(&Action::Put { count: 1 });
        let document = &editor.buffer_manager.active().doc;
        assert_eq!(document.buffer().row_text(0), "abc");
        assert_eq!(document.buffer().row_text(1), "abc");
    }

    #[test]
    fn test_join_lines() {
        let mut editor = Editor::new(Vec::new()).unwrap();
        editor.apply_active_action(&Action::InsertText("line 1\n  line 2\nline 3".into()));
        // Move back to line 1
        editor.apply_active_action(&Action::MoveUp {
            select: false,
            count: 2,
        });

        // Join line 1 and line 2
        editor.apply_active_action(&Action::JoinLines { count: 1 });

        let document = &editor.buffer_manager.active().doc;
        assert_eq!(document.buffer().row_text(0), "line 1 line 2");
        assert_eq!(document.buffer().row_text(1), "line 3");

        // Verify cursor is on the space
        assert_eq!(
            document.selection().head().to_point(document.buffer()),
            Point { row: 0, column: 6 }
        );
    }

    #[test]
    fn test_delete_around_character() {
        let mut editor = Editor::new(Vec::new()).unwrap();
        editor.apply_active_action(&Action::InsertText("a (hello) b".into()));
        // Move cursor inside parens
        editor.apply_active_action(&Action::MoveLeft {
            select: false,
            count: 7,
        });

        // Execute DeleteMotion around '('
        editor.apply_active_action(&Action::DeleteMotion {
            count: 1,
            motion: Box::new(Action::MoveAroundCharacter {
                count: 1,
                ch: '(',
            }),
        });

        let document = &editor.buffer_manager.active().doc;
        assert_eq!(document.buffer().row_text(0), "a  b");
    }

    #[test]
    fn test_delete_word() {
        let mut editor = Editor::new(Vec::new()).unwrap();
        editor.apply_active_action(&Action::InsertText("abc def".into()));
        editor.apply_active_action(&Action::MoveLeft {
            select: false,
            count: 7,
        });

        editor.apply_active_action(&Action::DeleteMotion {
            count: 1,
            motion: Box::new(Action::MoveToWord {
                count: 1,
                select: false,
            }),
        });

        let document = &editor.buffer_manager.active().doc;
        assert_eq!(document.buffer().row_text(0), "def");
    }

    #[test]
    fn test_delete_inner_word() {
        let mut editor = Editor::new(Vec::new()).unwrap();
        editor.apply_active_action(&Action::InsertText("abc def ghi".into()));
        // Move to 'e' in 'def'
        editor.apply_active_action(&Action::MoveLeft {
            select: false,
            count: 6,
        });

        // diw
        editor.apply_active_action(&Action::DeleteMotion {
            count: 1,
            motion: Box::new(Action::MoveWithinCharacter {
                count: 1,
                ch: 'w',
            }),
        });

        let document = &editor.buffer_manager.active().doc;
        assert_eq!(document.buffer().row_text(0), "abc  ghi");
    }

    #[test]
    fn test_delete_around_word() {
        let mut editor = Editor::new(Vec::new()).unwrap();
        editor.apply_active_action(&Action::InsertText("abc def ghi".into()));
        // Move to 'e' in 'def'
        editor.apply_active_action(&Action::MoveLeft {
            select: false,
            count: 6,
        });

        // daw
        editor.apply_active_action(&Action::DeleteMotion {
            count: 1,
            motion: Box::new(Action::MoveAroundCharacter {
                count: 1,
                ch: 'w',
            }),
        });

        let document = &editor.buffer_manager.active().doc;
        assert_eq!(document.buffer().row_text(0), "abc ghi");
    }

    #[test]
    fn test_treesitter_folding() {
        let mut editor = Editor::new(Vec::new()).unwrap();
        let text = "fn main() {\n    let x = 1;\n    let y = 2;\n}";
        editor.buffer_manager.active_mut().doc = Document::new_with_text(text);
        editor.buffer_manager.active_mut().grammar = Some(Grammar::Rust);
        
        let mut parser = TreeSitterParser::new(Grammar::Rust).unwrap();
        let tree = parser.parse(editor.buffer_manager.active().doc.buffer().snapshot(), None).unwrap();
        editor.buffer_manager.active_mut().syntax_tree = Some(tree);

        editor.apply_active_action(&Action::MoveDown { select: false, count: 1 });

        editor.apply_active_action(&Action::Fold { count: 1 });

        let active_buffer = editor.buffer_manager.active();
        assert_eq!(active_buffer.doc.folds.len(), 1);
        let fold = &active_buffer.doc.folds[0];
        assert_eq!(fold.start.row, 0);
        assert_eq!(fold.start.column, 11);
        assert_eq!(fold.end.row, 3);
        assert_eq!(fold.end.column, 0);

        editor.apply_active_action(&Action::Unfold { count: 1 });
        let active_buffer = editor.buffer_manager.active();
        assert_eq!(active_buffer.doc.folds.len(), 0);
    }

    #[test]
    fn test_fold_deletion() {
        let mut editor = Editor::new(Vec::new()).unwrap();
        let text = "line 1\nline 2\nline 3\nline 4";
        editor.buffer_manager.active_mut().doc = Document::new_with_text(text);
        
        let fold = crate::display::fold_map::Fold {
            start: Point::new(1, 0),
            end: Point::new(2, 6),
        };
        editor.buffer_manager.active_mut().doc.folds.push(fold);
        assert_eq!(editor.buffer_manager.active().doc.folds.len(), 1);

        editor.apply_active_action(&Action::MoveDown { select: false, count: 1 });
        editor.apply_active_action(&Action::Delete { count: 1 });

        assert_eq!(editor.buffer_manager.active().doc.folds.len(), 0);
    }
}
