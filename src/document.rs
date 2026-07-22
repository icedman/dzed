use crate::actions::Mode::Insert;
use crate::actions::{Action, Mode, SelectInKind};
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

        Ok(Self {
            buffer,
            selections,
            mode: Mode::Normal,
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

    pub fn enter_mode(&mut self, mode: Mode) {
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

    pub fn select_in(&mut self, kind: &SelectInKind) {
        // Select inside current word: start at word start, extend to word end
        self.selections
            .move_to_previous_word(false, 1, &self.buffer);
        self.selections.move_to_next_word_end(true, 1, &self.buffer);
    }

    pub fn apply_action(&mut self, action: &Action) {
        let mut next_action = Action::NoOp;
        match action {
            Action::InsertNewLineMotion { .. }
            | Action::Change
            | Action::ChangeCurrentLine { .. }
            | Action::ChangeMotion { .. } => next_action = Action::SetInsertMode,
            _ => {}
        }
        if self.mode == Mode::VisualBlock {
            match action {
                Action::Delete { .. } | Action::DeleteMotion { .. } => {
                    next_action = Action::SetInsertMode
                }
                _ => {}
            }
        }
        match action {
            Action::SetInsertMode => {
                self.enter_mode(Mode::Insert);
                return;
            }
            Action::SetInsertModeMotion { motion } => {
                self.enter_mode(Mode::Insert);
                next_action = (**motion).clone();
            }
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
                self.selections
                    .move_to_previous_word(*select, *count, &self.buffer)
            }
            Action::MoveToNextWord { select, count } => {
                self.selections
                    .move_to_next_word(*select, *count, &self.buffer)
            }
            Action::MoveToPreviousWordEnd { select, count } => self
                .selections
                .move_to_previous_word_end(*select, *count, &self.buffer),
            Action::MoveToNextWordEnd { select, count } => {
                self.selections
                    .move_to_next_word_end(*select, *count, &self.buffer)
            }
            Action::MoveToPreviousParagraph { select, count } => self
                .selections
                .move_to_previous_paragraph(*select, *count, &self.buffer),
            Action::MoveToNextParagraph { select, count } => self
                .selections
                .move_to_next_paragraph(*select, *count, &self.buffer),
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
            Action::MoveToPreviousMatch { search, pattern } => self
                .selections
                .move_to_previous_match(search, *pattern, &self.buffer),
            Action::MoveToNextMatch { search, pattern } => {
                self.selections
                    .move_to_next_match(search, *pattern, &self.buffer)
            }
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
            Action::MoveToStartOfPreviousLine { select } => self
                .selections
                .move_to_start_of_previous_line(*select, &self.buffer),
            Action::MoveToEndOfPreviousLine { select } => self
                .selections
                .move_to_end_of_previous_line(*select, &self.buffer),
            Action::MoveToStartOfNextLine { select } => self
                .selections
                .move_to_start_of_next_line(*select, &self.buffer),
            Action::MoveToEndOfNextLine { select } => self
                .selections
                .move_to_end_of_next_line(*select, &self.buffer),
            Action::MoveToLine { select, line } => {
                self.selections.move_to_line(*select, *line, &self.buffer)
            }
            Action::InsertText(text) => {
                self.delete_text(0);
                self.insert_text(text);
            }
            Action::DeleteText { count } => {
                self.delete_text(*count);
            }
            Action::Backspace => {
                if self.delete_text(0) {
                    //
                } else {
                    self.selections.move_left(false, 1, &self.buffer);
                    self.delete_text(1);
                }
            }
            Action::Delete { count } => {
                if self.delete_text(0) {
                    //
                } else {
                    for _ in 0..*count {
                        self.delete_text(1);
                    }
                }
            }
            Action::ChangeCurrentLine { count } | Action::DeleteCurrentLine { count } => {
                self.delete_current_line(*count);
            }
            Action::ChangeMotion { count, motion } | Action::DeleteMotion { count, motion } => {
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
                } else {
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

                    for _ in 0..*count {
                        self.apply_action(&motion);
                        self.delete_text(0);
                    }
                }
            }
            Action::Change => {
                self.delete_text(0);
            }
            Action::InsertNewLine => {
                self.delete_text(0);
                self.insert_text(&self.new_line().to_string());
            }
            Action::InsertNewLineMotion { count, motion } => {
                let mut motion = (**motion).clone();
                for _ in 0..*count {
                    self.apply_action(&motion);
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
            Action::Undo { count } => self.undo(*count),
            Action::Redo { count } => self.redo(*count),
            Action::SelectIn { kind } => self.select_in(kind),
            Action::SelectAround { kind } => self.select_in(kind),
            Action::ClearCursors => self.selections.clear_selections(&self.buffer),
            &Action::Indent | &Action::Unindent => {}
            Action::NoOp => {
                return;
            }
            _ => {}
        }

        self.apply_action(&next_action);
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

    #[test]
    fn consecutive_insert_text_actions_leave_cursor_after_inserted_text() {
        let mut document = Document::new("").unwrap();
        document.enter_mode(Mode::Insert);
        document.apply_action(&Action::InsertText("abc".into()));
        document.apply_action(&Action::MoveLeft {
            select: false,
            count: 2,
        });

        document.apply_action(&Action::InsertText("x".into()));
        document.apply_action(&Action::InsertText("y".into()));

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
        let mut newline_document = Document::new("").unwrap();
        newline_document.enter_mode(Mode::Insert);
        newline_document.apply_action(&Action::InsertText("abc".into()));
        newline_document.apply_action(&Action::MoveLeft {
            select: false,
            count: 2,
        });
        newline_document.apply_action(&Action::InsertNewLine);

        assert_eq!(newline_document.buffer().row_text(0), "a");
        assert_eq!(newline_document.buffer().row_text(1), "bc");
        assert_eq!(
            newline_document
                .selection()
                .head()
                .to_point(newline_document.buffer()),
            Point::new(1, 0)
        );

        let mut tab_document = Document::new("").unwrap();
        tab_document.enter_mode(Mode::Insert);
        tab_document.apply_action(&Action::InsertText("abc".into()));
        tab_document.apply_action(&Action::MoveLeft {
            select: false,
            count: 2,
        });
        tab_document.apply_action(&Action::InsertTab);

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
}
