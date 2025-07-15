use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use text::Anchor;
use text::Buffer;
use text::BufferId;
use text::Selection;
use text::SelectionGoal;
use text::ToOffset;
use text::ToPoint;

use rope::Point;
use rope::Rope;
use sum_tree::Bias;

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

pub trait WordOffsets {
    fn words_with_offsets(&self) -> Vec<(usize, usize, &str)>;
    fn find_next_word(&self, position: usize) -> Option<(usize, usize, &str)>;
    fn find_previous_word(&self, position: usize) -> Option<(usize, usize, &str)>;
    fn find_current_word(&self, position: usize) -> Option<(usize, usize, &str)>;
}

impl WordOffsets for str {
    fn words_with_offsets(&self) -> Vec<(usize, usize, &str)> {
        let mut words = Vec::new();
        let mut in_word = false;
        let mut word_start = 0;

        for (idx, ch) in self.char_indices() {
            if ch.is_whitespace() {
                if in_word {
                    words.push((word_start, idx, &self[word_start..idx]));
                    in_word = false;
                }
            } else if !in_word {
                in_word = true;
                word_start = idx;
            }
        }

        if in_word {
            words.push((word_start, self.len(), &self[word_start..]));
        }

        words
    }

    /// Find the next word after the given position
    fn find_next_word(&self, position: usize) -> Option<(usize, usize, &str)> {
        let offsets = self.words_with_offsets();
        offsets
            .iter()
            .find(|(start, _end, str)| *start > position)
            .copied()
    }

    /// Find the previous word before the given position
    fn find_previous_word(&self, position: usize) -> Option<(usize, usize, &str)> {
        let offsets = self.words_with_offsets();
        offsets
            .iter()
            .rev()
            .find(|(_start, end, str)| *end < position)
            .copied()
    }

    /// Find the word that contains the given position
    fn find_current_word(&self, position: usize) -> Option<(usize, usize, &str)> {
        let offsets = self.words_with_offsets();
        offsets
            .iter()
            .find(|(start, end, _)| *start <= position && position < *end)
            .copied()
    }
}

#[derive(Clone, Debug)]
pub struct Cursor {
    pub id: usize,
    pub row: u32,
    pub col: u32,
    pub anchor_row: u32,
    pub anchor_col: u32,
    pub head: Anchor,
    pub tail: Anchor,
}

impl Cursor {
    pub fn compute(&mut self, buffer: &Buffer) {
        let hp = buffer.clip_point(self.head.to_point(buffer), self.head.bias);
        let ap = buffer.clip_point(self.tail.to_point(buffer), self.tail.bias);

        self.row = hp.row;
        self.col = hp.column;
        self.anchor_row = ap.row;
        self.anchor_col = ap.column;
    }

    pub fn has_selection(&self) -> bool {
        self.row != self.anchor_row || self.col != self.anchor_col
    }

    pub fn clear_selection(&mut self) {
        self.tail = self.head.clone();
    }

    pub fn is_within(&self, row: u32, col: u32) -> bool {
        let cur = self.normalized();
        if row < cur.anchor_row || row > cur.row {
            return false;
        }
        if row == cur.anchor_row && col < cur.anchor_col {
            return false;
        }
        if row == cur.row && col > cur.col {
            return false;
        }
        true
    }

    pub fn normalized(&self) -> Self {
        let (row, col, anchor_row, anchor_col) = if self.row < self.anchor_row
            || (self.row == self.anchor_row && self.col < self.anchor_col)
        {
            (self.anchor_row, self.anchor_col, self.row, self.col)
        } else {
            (self.row, self.col, self.anchor_row, self.anchor_col)
        };

        Self {
            id: self.id,
            row,
            col,
            anchor_row,
            anchor_col,
            head: self.head,
            tail: self.tail,
        }
    }
}

pub struct Document {
    file_path: String,
    buffer: Buffer,
    cursors: Vec<Cursor>,
    next_cursor_id: usize,
}

impl Document {
    pub fn new(file_path: &str) -> io::Result<Self> {
        let contents = std::fs::read_to_string(file_path)?;
        let buffer = Buffer::new(0, BufferId::new(1).unwrap(), contents);

        Ok(Self {
            file_path: file_path.to_string(),
            buffer,
            cursors: vec![Cursor {
                id: 0,
                row: 0,
                col: 0,
                anchor_row: 0,
                anchor_col: 0,
                head: Anchor::MIN,
                tail: Anchor::MIN,
            }],
            next_cursor_id: 1,
        })
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

    fn move_cursor<F>(&mut self, anchor: bool, movement: F)
    where
        F: Fn(&mut Cursor, &Buffer),
    {
        for cursor in &mut self.cursors {
            movement(cursor, &self.buffer);
            if !anchor {
                cursor.tail = cursor.head.clone();
            }
            cursor.compute(&self.buffer);
        }
    }

    pub fn move_left(&mut self, anchor: bool) {
        self.move_cursor(anchor, |cursor, buffer| {
            let mut offset = buffer.offset_for_anchor(&cursor.head);
            if offset > 0 {
                offset -= 1;
            }
            cursor.head = buffer.anchor_at(
                buffer.clip_offset(offset, cursor.head.bias),
                cursor.head.bias,
            );
        });
    }

    pub fn move_right(&mut self, anchor: bool) {
        self.move_cursor(anchor, |cursor, buffer| {
            let offset = buffer.offset_for_anchor(&cursor.head) + 1;
            cursor.head = buffer.anchor_at(
                buffer.clip_offset(offset, cursor.head.bias),
                cursor.head.bias,
            );
        });
    }

    pub fn move_up(&mut self, anchor: bool) {
        self.move_cursor(anchor, |cursor, buffer| {
            let mut point = cursor.head.to_point(buffer);
            if point.row > 0 {
                point.row -= 1;
            }
            point = buffer.clip_point(point, cursor.head.bias);
            let offset = point.to_offset(buffer);
            cursor.head = buffer.anchor_at(
                buffer.clip_offset(offset, cursor.head.bias),
                cursor.head.bias,
            );
        });
    }

    pub fn move_down(&mut self, anchor: bool) {
        self.move_cursor(anchor, |cursor, buffer| {
            let mut point = cursor.head.to_point(buffer);
            point.row += 1;
            point = buffer.clip_point(point, cursor.head.bias);
            let offset = point.to_offset(buffer);
            cursor.head = buffer.anchor_at(
                buffer.clip_offset(offset, cursor.head.bias),
                cursor.head.bias,
            );
        });
    }

    pub fn move_to_previous_word(&mut self, anchor: bool) {
        self.move_cursor(anchor, |cursor, buffer| {
            let mut point = cursor.head.to_point(buffer);
            let text = buffer.row_text(point.row);
            if let Some(word) = text.find_previous_word(point.column as usize) {
                // Found previous word
                let (start, end, w) = word;
                point.column = start as u32;
            } else {
                point.column = 0;
            }
            let offset = point.to_offset(buffer);
            cursor.head = buffer.anchor_at(
                buffer.clip_offset(offset, cursor.head.bias),
                cursor.head.bias,
            );
        });
    }

    pub fn move_to_next_word(&mut self, anchor: bool) {
        self.move_cursor(anchor, |cursor, buffer| {
            let mut point = cursor.head.to_point(buffer);
            let text = buffer.row_text(point.row);
            if let Some(word) = text.find_next_word(point.column as usize) {
                // Found next word
                let (start, end, w) = word;
                point.column = (end - 1) as u32;
            } else {
                point.column = buffer.line_len(point.row);
            }
            let offset = point.to_offset(buffer);
            cursor.head = buffer.anchor_at(
                buffer.clip_offset(offset, cursor.head.bias),
                cursor.head.bias,
            );
        });
    }

    pub fn move_to_start_of_line(&mut self, anchor: bool) {
        self.move_cursor(anchor, |cursor, buffer| {
            let mut point = cursor.head.to_point(buffer);
            point.column = 0;
            let offset = point.to_offset(buffer);
            cursor.head = buffer.anchor_at(
                buffer.clip_offset(offset, cursor.head.bias),
                cursor.head.bias,
            );
        });
    }

    pub fn move_to_end_of_line(&mut self, anchor: bool) {
        self.move_cursor(anchor, |cursor, buffer| {
            let mut point = cursor.head.to_point(buffer);
            point.column = buffer.line_len(point.row);
            let offset = point.to_offset(buffer);
            cursor.head = buffer.anchor_at(
                buffer.clip_offset(offset, cursor.head.bias),
                cursor.head.bias,
            );
        });
    }

    pub fn move_to_start_of_document(&mut self, anchor: bool) {
        self.move_cursor(anchor, |cursor, buffer| {
            let mut point = cursor.head.to_point(buffer);
            point.row = 0;
            point.column = 0;
            let offset = point.to_offset(buffer);
            cursor.head = buffer.anchor_at(
                buffer.clip_offset(offset, cursor.head.bias),
                cursor.head.bias,
            );
        });
    }

    pub fn move_to_end_of_document(&mut self, anchor: bool) {
        self.move_cursor(anchor, |cursor, buffer| {
            let mut point = cursor.head.to_point(buffer);
            point.row = buffer.row_count();
            point.column = 0;
            let offset = point.to_offset(buffer);
            cursor.head = buffer.anchor_at(
                buffer.clip_offset(offset, cursor.head.bias),
                cursor.head.bias,
            );
        });
    }

    pub fn select_current_word(&mut self) {
        for cursor in &mut self.cursors {
            if !cursor.has_selection() {
                let point = cursor.head.to_point(&self.buffer);
                let text = self.buffer.row_text(point.row);
                if let Some(word) = text.find_current_word(point.column as usize) {
                    // Found next word
                    let (start, end, w) = word;
                    {
                        let mut p = point.clone();
                        p.column = end as u32;
                        let offset = p.to_offset(&self.buffer);
                        cursor.head = self.buffer.anchor_at(
                            self.buffer.clip_offset(offset, cursor.head.bias),
                            Bias::Right,
                        );
                    }

                    {
                        let mut p = point.clone();
                        p.column = start as u32;
                        let offset = p.to_offset(&self.buffer);
                        cursor.tail = self.buffer.anchor_at(
                            self.buffer.clip_offset(offset, cursor.head.bias),
                            Bias::Left,
                        );
                    }
                    cursor.compute(&self.buffer);
                }
            }
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        for cursor in &mut self.cursors {
            let cur = cursor.normalized();
            let offset = Point::new(cur.row, cur.col).to_offset(&self.buffer);
            self.buffer.edit([(offset..offset, text)]);
        }
    }

    pub fn delete_text(&mut self, count: usize) {
        for cursor in &mut self.cursors {
            let cur = cursor.normalized();
            let mut start = Point::new(cur.anchor_row, cur.anchor_col).to_offset(&self.buffer);
            let mut end = Point::new(cur.row, cur.col).to_offset(&self.buffer);

            if start == end {
                end += count;
            }

            self.buffer.edit([(start..end, "")]);
            cursor.compute(&self.buffer);
        }
    }

    pub fn row_text(&self, row: u32) -> String {
        return self.buffer.row_text(row);
    }

    pub fn cursor(&self, id: usize) -> Option<&Cursor> {
        self.cursors.iter().find(|c| c.id == id)
    }
}
