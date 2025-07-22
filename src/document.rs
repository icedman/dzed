use crate::actions::Action;
use crate::selections::SelectionCollection;
use rope::Point;
use std::cmp::Ordering;
use std::io;
use std::ops::Range;
use sum_tree::Bias;
use text::{Anchor, Buffer, BufferId, Selection, SelectionGoal, ToOffset, ToPoint};

pub fn string_to_byte_indices(text: &str) -> Vec<usize> {
    let mut indices = Vec::new();
    let mut idx = 0;
    for c in text.chars() {
        indices.push(idx);
        idx += c.len_utf8();
    }
    // pad
    for _i in 0..16 {
        indices.push(idx);
    }
    indices
}

pub fn string_to_byte_sizes(text: &str) -> Vec<usize> {
    let mut sizes = Vec::new();
    for c in text.chars() {
        sizes.push(c.len_utf8());
    }
    // pad
    for _i in 0..16 {
        sizes.push(0);
    }
    sizes
}

pub trait BufferText {
    fn row_text(&self, row: u32) -> String;
    fn row_len(&self, row: u32) -> u32;
}

impl BufferText for Buffer {
    fn row_text(&self, row: u32) -> String {
        let start = Point::new(row, 0).to_offset(self);
        let end = Point::new(row, self.line_len(row)).to_offset(self);
        self.as_rope().chunks_in_range(start..end).collect()
    }

    fn row_len(&self, row: u32) -> u32 {
        let l = self.line_len(row).saturating_sub(1) as usize;
        let text = self.row_text(row);
        let indices = string_to_byte_sizes(&text);
        let sizes = string_to_byte_sizes(&text);
        (indices[l] + sizes[l]) as u32
    }
}

pub trait WordOffsets {
    fn words_with_offsets(&self) -> Vec<(usize, usize, &str)>;
    fn find_next_word(&self, position: usize) -> Option<(usize, usize, &str)>;
    fn find_previous_word(&self, position: usize) -> Option<(usize, usize, &str)>;
    fn find_current_word(&self, position: usize) -> Option<(usize, usize, &str)>;
    fn find_next_same_word(&self, position: usize, word: &str) -> Option<(usize, usize, &str)>;
}

impl WordOffsets for str {
    fn words_with_offsets(&self) -> Vec<(usize, usize, &str)> {
        let mut words = Vec::new();
        let mut in_word = false;
        let mut word_start = 0;

        for (idx, ch) in self.char_indices() {
            if ch.is_alphanumeric() {
                if !in_word {
                    in_word = true;
                    word_start = idx;
                }
            } else if in_word {
                words.push((word_start, idx, &self[word_start..idx]));
                in_word = false;
            }
        }

        if in_word {
            words.push((word_start, self.len(), &self[word_start..]));
        }

        words
    }

    fn find_next_word(&self, position: usize) -> Option<(usize, usize, &str)> {
        self.words_with_offsets()
            .into_iter()
            .find(|(start, _, _)| *start > position)
    }

    fn find_previous_word(&self, position: usize) -> Option<(usize, usize, &str)> {
        self.words_with_offsets()
            .into_iter()
            .rev()
            .find(|(_, end, _)| *end < position)
    }

    fn find_current_word(&self, position: usize) -> Option<(usize, usize, &str)> {
        self.words_with_offsets()
            .into_iter()
            .find(|(start, end, _)| *start <= position && position < *end)
    }

    fn find_next_same_word(&self, position: usize, word: &str) -> Option<(usize, usize, &str)> {
        self.words_with_offsets()
            .into_iter()
            .skip_while(|(start, _, _)| *start <= position)
            .find(|(_, _, w)| *w == word)
    }
}

pub struct Document {
    buffer: Buffer,
    selections: SelectionCollection,
}

impl Document {
    pub fn new(file_path: &str) -> io::Result<Self> {
        // let contents = std::fs::read_to_string(file_path)?;
        let contents = if std::path::Path::new(file_path).exists() {
            match std::fs::read_to_string(file_path) {
                Ok(s) => s,
                Err(_) => "File not found".to_string(),
            }
        } else {
            " ".to_string()
        };
        let buffer = Buffer::new(0, BufferId::new(1).unwrap(), contents);
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

    pub fn undo(&mut self) {
        self.buffer.undo();
    }

    pub fn redo(&mut self) {
        self.buffer.redo();
    }

    pub fn move_left(&mut self, anchor: bool) {
        let mut cursor = self.first_selection();
        let point = cursor.head().to_point(&self.buffer);
        let (v, l) = {
            let row_text = self.buffer.row_text(point.row);
            (string_to_byte_sizes(&row_text), row_text.len())
        };
        let mut offset = self
            .buffer
            .offset_for_anchor(&cursor.head())
            .saturating_sub(v[point.column.saturating_sub(1) as usize]);
        offset = self.buffer.clip_offset(offset, Bias::Left);
        let new_head = self.buffer.anchor_at(offset, Bias::Left);
        self.selections.update(&{
            Selection {
                id: cursor.id,
                start: new_head,
                end: if anchor { cursor.tail() } else { new_head },
                reversed: true,
                goal: SelectionGoal::None,
            }
        });
    }

    pub fn move_right(&mut self, anchor: bool) {
        let mut cursor = self.first_selection();
        let point = cursor.head().to_point(&self.buffer);
        let (v, l) = {
            let row_text = self.buffer.row_text(point.row);
            (string_to_byte_sizes(&row_text), row_text.len())
        };
        let mut offset = self.buffer.offset_for_anchor(&cursor.head()) + v[point.column as usize];
        offset = self.buffer.clip_offset(offset, Bias::Left);
        let new_head = self.buffer.anchor_at(offset, Bias::Left);
        self.selections.update(&{
            Selection {
                id: cursor.id,
                start: new_head,
                end: if anchor { cursor.tail() } else { new_head },
                reversed: true,
                goal: SelectionGoal::None,
            }
        });
    }

    pub fn move_up(&mut self, anchor: bool) {
        let cursor = self.first_selection();
        let mut point = cursor.head().to_point(&self.buffer);
        point.row = point.row.saturating_sub(1);
        point = self.buffer.clip_point(point, cursor.head().bias);
        let mut offset = point.to_offset(&self.buffer);
        let new_head = self.buffer.anchor_at(offset, Bias::Left);
        self.selections.update(&{
            Selection {
                id: cursor.id,
                start: new_head,
                end: if anchor { cursor.tail() } else { new_head },
                reversed: true,
                goal: SelectionGoal::None,
            }
        });
    }

    pub fn move_down(&mut self, anchor: bool) {
        let cursor = self.first_selection();
        let mut point = cursor.head().to_point(&self.buffer);
        point.row += 1;
        point = self.buffer.clip_point(point, cursor.head().bias);
        let mut offset = point.to_offset(&self.buffer);
        let new_head = self.buffer.anchor_at(offset, Bias::Left);
        self.selections.update(&{
            Selection {
                id: cursor.id,
                start: new_head,
                end: if anchor { cursor.tail() } else { new_head },
                reversed: true,
                goal: SelectionGoal::None,
            }
        });
    }

    pub fn move_to_previous_word(&mut self, anchor: bool) {
        // self.move_cursor(anchor, |cursor, buffer| {
        // let mut point = cursor.head().to_point(buffer);
        // let text = buffer.row_text(point.row);
        // if let Some(word) = text.find_previous_word(point.column as usize) {
        //     // Found previous word
        //     let (start, _end, _w) = word;
        //     point.column = start as u32;
        // } else {
        //     point.column = 0;
        // }
        // let offset = point.to_offset(buffer);
        // cursor.head = buffer.anchor_at(
        //     buffer.clip_offset(offset, cursor.head().bias),
        //     cursor.head().bias,
        // );
        // });
    }

    pub fn move_to_next_word(&mut self, anchor: bool) {
        // self.move_cursor(anchor, |cursor, buffer| {
        // let mut point = cursor.head().to_point(buffer);
        // let text = buffer.row_text(point.row);
        // if let Some(word) = text.find_next_word(point.column as usize) {
        //     // Found next word
        //     let (_start, end, _w) = word;
        //     point.column = (end - 1) as u32;
        // } else {
        //     point.column = buffer.line_len(point.row);
        // }
        // let offset = point.to_offset(buffer);
        // cursor.head = buffer.anchor_at(
        //     buffer.clip_offset(offset, cursor.head().bias),
        //     cursor.head().bias,
        // );
        // });
    }

    pub fn move_to_start_of_line(&mut self, anchor: bool) {
        let cursor = self.first_selection();
        let mut point = cursor.head().to_point(&self.buffer);
        point.column = 0;
        point = self.buffer.clip_point(point, cursor.head().bias);
        let mut offset = point.to_offset(&self.buffer);
        let new_head = self.buffer.anchor_at(offset, Bias::Left);
        self.selections.update(&{
            Selection {
                id: cursor.id,
                start: new_head,
                end: if anchor { cursor.tail() } else { new_head },
                reversed: true,
                goal: SelectionGoal::None,
            }
        });
    }

    pub fn move_to_end_of_line(&mut self, anchor: bool) {
        let cursor = self.first_selection();
        let mut point = cursor.head().to_point(&self.buffer);
        point.column = self.buffer.line_len(point.row);
        point = self.buffer.clip_point(point, cursor.head().bias);
        let mut offset = point.to_offset(&self.buffer);
        let new_head = self.buffer.anchor_at(offset, Bias::Left);
        self.selections.update(&{
            Selection {
                id: cursor.id,
                start: new_head,
                end: if anchor { cursor.tail() } else { new_head },
                reversed: true,
                goal: SelectionGoal::None,
            }
        });
    }

    pub fn move_to_start_of_document(&mut self, anchor: bool) {
        // self.move_cursor(anchor, |cursor, buffer| {
        // let mut point = cursor.head().to_point(buffer);
        // point.row = 0;
        // point.column = 0;
        // let offset = point.to_offset(buffer);
        // cursor.head = buffer.anchor_at(
        //     buffer.clip_offset(offset, cursor.head().bias),
        //     cursor.head().bias,
        // );
        // });
    }

    pub fn move_to_end_of_document(&mut self, anchor: bool) {
        // self.move_cursor(anchor, |cursor, buffer| {
        // let mut point = cursor.head().to_point(buffer);
        // point.row = buffer.row_count();
        // point.column = 0;
        // let offset = point.to_offset(buffer);
        // cursor.head = buffer.anchor_at(
        //     buffer.clip_offset(offset, cursor.head().bias),
        //     cursor.head().bias,
        // );
        // });
    }

    pub fn insert_text(&mut self, text: &str) {
        let cursor = self.first_selection();
        let start = self.buffer.offset_for_anchor(&cursor.head());
        self.buffer.edit([(start..start, text)]);
    }

    pub fn delete_text(&mut self, count: usize) {
        let cursor = self.first_selection();
        let (start, mut end) = {
            let start = self.buffer.offset_for_anchor(&cursor.head());
            let end = self.buffer.offset_for_anchor(&cursor.tail());
            if cursor.head().cmp(&cursor.tail(), &self.buffer) == Ordering::Less {
                (start, end)
            } else {
                (end, start)
            }
        };

        if start == end && count != 0 {
            let point = self.buffer.offset_to_point(start);
            let (v, l) = {
                let row_text = self.buffer.row_text(point.row);
                (string_to_byte_sizes(&row_text), row_text.len())
            };
            end = self
                .buffer
                .clip_offset(start + v[point.column as usize], Bias::Left);
        }

        self.buffer.edit([(start..end, "")]);
    }

    pub fn row_text(&self, row: u32) -> String {
        return self.buffer.row_text(row);
    }

    pub fn select_current_word(&mut self) {
        // let cursor = self.cursor(0).unwrap().clone();
        // if cursor.has_selection() {
        //     let sel = cursor.selection_text(&self.buffer);
        //     return;
        // }

        // for cursor in &mut self.cursors {
        //     if !cursor.has_selection() {
        //         let point = cursor.head().to_point(&self.buffer);
        //         let text = self.buffer.row_text(point.row);
        //         if let Some(word) = text.find_current_word(point.column as usize) {
        //             // Found next word
        //             let (start, end, _w) = word;
        //             {
        //                 let mut p = point.clone();
        //                 p.column = end as u32;
        //                 let offset = p.to_offset(&self.buffer);
        //                 // cursor.head = self.buffer.anchor_at(
        //                 //     self.buffer.clip_offset(offset, cursor.head().bias),
        //                 //     Bias::Right,
        //                 // );
        //             }

        //             {
        //                 let mut p = point.clone();
        //                 p.column = start as u32;
        //                 let offset = p.to_offset(&self.buffer);
        //                 // cursor.tail = self.buffer.anchor_at(
        //                 //     self.buffer.clip_offset(offset, cursor.head().bias),
        //                 //     Bias::Left,
        //                 // );
        //             }
        //             cursor.compute(&self.buffer);
        //         }
        //     }
        // }
    }

    pub fn select_next_same_word(&mut self, text: &str) {}

    pub fn apply_action(&mut self, action: &Action) {
        match action {
            Action::MoveUp { select, count } => {
                for _ in 0..*count {
                    self.move_up(*select);
                }
            }
            Action::MoveDown { select, count } => {
                for _ in 0..*count {
                    self.move_down(*select);
                }
            }
            Action::MoveLeft { select } => self.move_left(*select),
            Action::MoveRight { select } => self.move_right(*select),
            Action::MoveToPreviousWord { select } => self.move_to_previous_word(*select),
            Action::MoveToNextWord { select } => self.move_to_next_word(*select),
            Action::MoveToStartOfDocument { select } => self.move_to_start_of_document(*select),
            Action::MoveToEndOfDocument { select } => self.move_to_end_of_document(*select),
            Action::MoveToStartOfLine { select } => self.move_to_start_of_line(*select),
            Action::MoveToEndOfLine { select } => self.move_to_end_of_line(*select),
            Action::InsertText(text) => {
                self.delete_text(0);
                self.insert_text(text);
                self.move_right(false);
            }
            Action::DeleteText { count } => self.delete_text(*count),
            Action::Backspace => {
                self.delete_text(0);
                self.move_left(false);
                self.delete_text(1);
            }
            Action::Delete => {
                self.delete_text(0);
                self.delete_text(1);
            }
            Action::InsertNewLine => {
                self.delete_text(0);
                self.insert_text(&self.new_line().to_string());
                self.move_right(false);
            }
            Action::InsertTab => {
                for _ in 0..4 {
                    self.insert_text(" ");
                    self.move_right(false);
                }
            }
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::SelectCurrentWord => self.select_current_word(),
            Action::SelectNextSameWord(sel) => self.select_next_same_word(&sel),
            Action::ClearCursors => {}
            &Action::Indent | &Action::Unindent => {}

            Action::NoOp => {}
        }
    }

    pub fn first_selection(&self) -> Selection<Anchor> {
        self.selections.first().unwrap().clone()
    }

    pub fn add_selection(&mut self) -> Selection<Anchor> {
        return self.selections.add(&self.buffer, 0);
    }
}
