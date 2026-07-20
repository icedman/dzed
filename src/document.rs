use crate::actions::{Action, SelectInKind};
use crate::search::TextSearch;
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
}

impl Document {
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

        Ok(Self { buffer, selections })
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

    pub fn select_in(&mut self, kind: &SelectInKind) {
        self.move_to_previous_word(false, 1);
        self.move_to_next_word_end(true, 1);
    }

    pub fn apply_action(&mut self, action: &Action) {
        match action {
            Action::MoveUp { select, count } => {
                self.selections.move_up(*select, *count, &self.buffer)
            }
            Action::MoveDown { select, count } => {
                self.selections.move_down(*select, *count, &self.buffer)
            }
            Action::MoveLeft { select, count } => {
                self.selections.move_left(*select, *count, &self.buffer)
            }
            Action::MoveRight { select, count } => {
                self.selections.move_right(*select, *count, &self.buffer)
            }

            Action::MoveToPreviousWord { select, count } => {
                self.move_to_previous_word(*select, *count)
            }
            Action::MoveToNextWord { select, count } => self.move_to_next_word(*select, *count),
            Action::MoveToPreviousWordEnd { select, count } => {
                self.move_to_previous_word_end(*select, *count)
            }
            Action::MoveToNextWordEnd { select, count } => {
                self.move_to_next_word_end(*select, *count)
            }
            Action::MoveToPreviousParagraph { select, count } => {
                self.move_to_previous_paragraph(*select, *count)
            }
            Action::MoveToNextParagraph { select, count } => {
                self.move_to_next_paragraph(*select, *count)
            }
            Action::MoveToPreviousCharacter {
                select,
                count,
                char,
            } => self
                .selections
                .find_character(*select, *count, *char, false, &self.buffer),
            Action::MoveToNextCharacter {
                select,
                count,
                char,
            } => self
                .selections
                .find_character(*select, *count, *char, true, &self.buffer),
            Action::MoveToStartOfDocument { select } => self
                .selections
                .move_to_start_of_document(*select, &self.buffer),
            Action::MoveToEndOfDocument { select } => self
                .selections
                .move_to_end_of_document(*select, &self.buffer),
            Action::MoveToStartOfLine { select } => {
                self.selections.move_to_start_of_line(*select, &self.buffer)
            }
            Action::MoveToStartOfLineNonSpace { select } => self
                .selections
                .move_to_start_of_line_non_space(*select, &self.buffer),
            Action::MoveToEndOfLine { select } => {
                self.selections.move_to_end_of_line(*select, &self.buffer)
            }
            Action::MoveToLine { select, line } => {
                self.selections.move_to_line(*select, *line, &self.buffer)
            }
            Action::InsertText(text) => {
                let len = text.chars().count() as u32;
                self.delete_text(0);
                self.insert_text(text);
                self.selections.move_right(false, len, &self.buffer);
            }
            Action::DeleteText { count } => {
                self.delete_text(*count);
            }
            Action::Backspace => {
                if self.delete_text(0) {
                    return;
                }
                self.selections.move_left(false, 1, &self.buffer);
                self.delete_text(1);
            }
            Action::Delete { count } => {
                if self.delete_text(0) {
                    return;
                }
                for _ in 0..*count {
                    self.delete_text(1);
                }
            }
            Action::DeleteCurrentLine { count } => {
                self.delete_current_line(*count);
            }
            Action::DeleteMotion { count, motion } => {
                let mut motion = (**motion).clone();
                let is_textobject = match &motion {
                    Action::MoveToNextWord { .. }
                    | Action::MoveToNextParagraph { .. }
                    | Action::MoveToEndOfLine { .. } => true,
                    _ => false,
                };

                if is_textobject {
                    for idx in 0..*count {
                        self.apply_action(&motion);
                        self.delete_text_object();
                    }
                    return;
                }

                match &mut motion {
                    Action::MoveUp { select, .. }
                    | Action::MoveDown { select, .. }
                    | Action::MoveLeft { select, .. }
                    | Action::MoveRight { select, .. }
                    | Action::MoveToPreviousWord { select, .. }
                    | Action::MoveToNextWord { select, .. }
                    | Action::MoveToPreviousWordEnd { select, .. }
                    | Action::MoveToNextWordEnd { select, .. }
                    | Action::MoveToStartOfDocument { select }
                    | Action::MoveToEndOfDocument { select }
                    | Action::MoveToStartOfLine { select }
                    | Action::MoveToStartOfLineNonSpace { select }
                    | Action::MoveToEndOfLine { select }
                    | Action::MoveToLine { select, .. }
                    | Action::MoveToPreviousParagraph { select, .. }
                    | Action::MoveToNextParagraph { select, .. }
                    | Action::MoveToPreviousCharacter { select, .. }
                    | Action::MoveToNextCharacter { select, .. } => *select = true,
                    _ => {}
                }

                for idx in 0..*count {
                    self.apply_action(&motion);
                    self.delete_text(0);
                }
            }
            Action::InsertNewLine => {
                self.delete_text(0);
                self.insert_text(&self.new_line().to_string());
                self.selections.move_right(false, 1, &self.buffer);
            }
            Action::InsertTab => {
                for _ in 0..4 {
                    self.insert_text(" ");
                    self.selections.move_right(false, 1, &self.buffer);
                }
            }
            Action::Undo { count } => self.undo(*count),
            Action::Redo { count } => self.redo(*count),
            Action::SelectIn { kind } => self.select_in(kind),
            Action::ClearCursors => self.selections.clear_selections(&self.buffer),
            &Action::Indent | &Action::Unindent => {}

            Action::NoOp => {}
            _ => {}
        }
    }

    fn insert_text(&mut self, text: &str) {
        let cursors = self.selections.selections.clone();
        for cursor in cursors.iter() {
            let start = self.buffer.offset_for_anchor(&cursor.head());
            self.buffer.edit([(start..start, text)]);
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
                self.buffer.edit([(start..end, "")]);
            }
        }
        return delete_count > 0;
    }

    fn delete_text_object(&mut self) -> bool {
        let mut delete_count = 0;
        let cursors = self.selections.selections.clone();
        for cursor in cursors.iter() {
            let (start, end) = {
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
                let end = self.buffer.offset_for_anchor(&ce);
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
                    self.buffer.edit([(start..end, "")]);
                }
            }
        }
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

    pub fn has_selection(&self) -> bool {
        self.selections.has_selection(&self.buffer)
    }

    pub fn move_to_previous_word(&mut self, select: bool, count: u32) {
        for _ in 0..count {
            let cursors = self.selections.selections.clone();
            for cursor in cursors.iter() {
                let mut point = cursor.head().to_point(&self.buffer);
                let text = self.buffer.row_text(point.row);

                let previous_column = point.column;

                if let Some(word) = text.find_previous_word(point.column as usize) {
                    point.column = word.0 as u32;
                } else {
                    point.column = 0;
                }

                if point.column == previous_column {
                    self.selections
                        .update(&self.buffer, &cursor.move_left_once(select, &self.buffer));
                } else {
                    let offset = point.to_offset(&self.buffer);
                    let new_head = self.buffer.anchor_at(offset, Bias::Left);
                    self.selections.update(&self.buffer, &{
                        Selection {
                            id: cursor.id,
                            start: new_head,
                            end: if select { cursor.tail() } else { new_head },
                            reversed: true,
                            goal: SelectionGoal::None,
                        }
                    });
                }
            }
        }
    }

    pub fn move_to_next_word(&mut self, select: bool, count: u32) {
        for _ in 0..count {
            let cursors = self.selections.selections.clone();
            for cursor in cursors.iter() {
                let mut point = cursor.head().to_point(&self.buffer);
                let text = self.buffer.row_text(point.row);

                let previous_column = point.column;

                if let Some(word) = text.find_next_word(point.column as usize) {
                    point.column = word.0 as u32;
                } else {
                    point.column = self.buffer.line_len(point.row);
                }

                if point.column == previous_column {
                    self.selections
                        .update(&self.buffer, &cursor.move_right_once(select, &self.buffer));
                } else {
                    let mut offset = point.to_offset(&self.buffer);
                    offset = self.buffer.clip_offset(offset, Bias::Left);
                    let new_head = self.buffer.anchor_at(offset, Bias::Right);

                    self.selections.update(&self.buffer, &{
                        Selection {
                            id: cursor.id,
                            end: new_head,
                            start: if select { cursor.tail() } else { new_head },
                            reversed: false,
                            goal: SelectionGoal::None,
                        }
                    });
                }
            }
        }
    }

    pub fn move_to_next_word_end(&mut self, select: bool, count: u32) {
        for _ in 0..count {
            let cursors = self.selections.selections.clone();
            for cursor in cursors.iter() {
                let mut point = cursor.head().to_point(&self.buffer);
                let text = self.buffer.row_text(point.row);

                if let Some(word) = text.find_next_word_end(point.column as usize) {
                    point.column = (word.1 - 1) as u32;
                } else {
                    point.column = self.buffer.line_len(point.row); //.saturating_sub(1);
                }

                let mut offset = point.to_offset(&self.buffer);
                offset = self.buffer.clip_offset(offset, Bias::Left);
                let new_head = self.buffer.anchor_at(offset, Bias::Left);
                self.selections.update(&self.buffer, &{
                    Selection {
                        id: cursor.id,
                        end: new_head,
                        start: if select { cursor.tail() } else { new_head },
                        reversed: false,
                        goal: SelectionGoal::None,
                    }
                });
            }
        }
    }

    pub fn move_to_previous_word_end(&mut self, select: bool, count: u32) {
        for _ in 0..count {
            let cursors = self.selections.selections.clone();
            for cursor in cursors.iter() {
                let mut point = cursor.head().to_point(&self.buffer);
                let text = self.buffer.row_text(point.row);

                if let Some(word) = text.find_previous_word_end(point.column as usize) {
                    point.column = (word.1 - 1) as u32;
                } else {
                    point.column = 0;
                }

                let offset = point.to_offset(&self.buffer);
                let new_head = self.buffer.anchor_at(offset, Bias::Left);
                self.selections.update(&self.buffer, &{
                    Selection {
                        id: cursor.id,
                        start: new_head,
                        end: if select { cursor.tail() } else { new_head },
                        reversed: true,
                        goal: SelectionGoal::None,
                    }
                });
            }
        }
    }

    pub fn move_to_previous_paragraph(&mut self, select: bool, count: u32) {
        for _ in 0..count {
            let cursors = self.selections.selections.clone();
            for cursor in cursors.iter() {
                let mut point = cursor.head().to_point(&self.buffer);
                point.column = 0;
                let mut target_point = point.clone();
                let mut has_target = false;
                while point.row > 0 {
                    point.row -= 1;
                    if self.buffer.line_len(point.row) == 0 {
                        target_point = point.clone();
                        has_target = true;
                    } else if has_target {
                        break;
                    }
                }
                if has_target {
                    let mut offset = target_point.to_offset(&self.buffer);
                    offset = self.buffer.clip_offset(offset, Bias::Right);
                    let new_head = self.buffer.anchor_at(offset, Bias::Left);
                    self.selections.update(&self.buffer, &{
                        Selection {
                            id: cursor.id,
                            start: new_head,
                            end: if select { cursor.tail() } else { new_head },
                            reversed: true,
                            goal: SelectionGoal::None,
                        }
                    });
                }
            }
        }
    }

    pub fn move_to_next_paragraph(&mut self, select: bool, count: u32) {
        for _ in 0..count {
            let cursors = self.selections.selections.clone();
            for cursor in cursors.iter() {
                let mut point = cursor.head().to_point(&self.buffer);
                point.column = 0;
                let mut target_point = point.clone();
                let mut has_target = false;
                while point.row < self.buffer.row_count() {
                    point.row += 1;
                    if self.buffer.line_len(point.row) == 0 {
                        target_point = point.clone();
                        has_target = true;
                    } else if has_target {
                        break;
                    }
                }
                if has_target {
                    let offset = target_point.to_offset(&self.buffer);
                    let new_head = self.buffer.anchor_at(offset, Bias::Left);
                    self.selections.update(&self.buffer, &{
                        Selection {
                            id: cursor.id,
                            start: new_head,
                            end: if select { cursor.tail() } else { new_head },
                            reversed: true,
                            goal: SelectionGoal::None,
                        }
                    });
                }
            }
        }
    }
}
