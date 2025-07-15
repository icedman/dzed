use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use text::Buffer;
use text::BufferId;
use text::Selection;
use text::SelectionGoal;
use text::ToOffset;

use rope::Point;
use rope::Rope;

#[derive(Clone, Debug)]
pub struct Cursor {
    pub id: usize,
    pub row: u32,
    pub col: u32,
    pub anchor_row: u32,
    pub anchor_col: u32,
}

impl Cursor {
    pub fn is_within(&self, row: u32, col: u32) -> bool {
        // assumes a normalized cursor
        if row < self.anchor_row {
            return false
        }
        if row > self.row {
            return false
        }
        if col < self.anchor_col && row == self.anchor_row{
            return false;
        }
        if col > self.col && row == self.row {
            return false;
        }
        true
    }

    pub fn has_selection(&self) -> bool {
        return self.row != self.anchor_row || self.col != self.anchor_col
    }

    pub fn normalized(&self) -> Cursor {
        let mut row = self.row;
        let mut col = self.col;
        let mut anchor_row = self.anchor_row;
        let mut anchor_col = self.anchor_col;
        if row < anchor_row {
            let tr = row;
            let tc = col;
            row = anchor_row;
            anchor_row = tr;
            col = anchor_col;
            anchor_col = tc;
        }
        if row == anchor_row && col < anchor_col {
            let tc = col;
            col = anchor_col;
            anchor_col = tc;
        }
        return Cursor {
            id: self.id,
            row,
            col,
            anchor_row,
            anchor_col,
        };
    }

    pub fn sane(&self, buffer: &Buffer) -> Cursor {
        let mut row = self.row;
        let mut col = self.col;
        let mut anchor_row = self.anchor_row;
        let mut anchor_col = self.anchor_col;
        let line_len = buffer.line_len(row);
        let anchor_line_len = buffer.line_len(anchor_row);
        if line_len == 0 {
            col = 0;
        }
        if anchor_line_len == 0 {
            anchor_col = 0;
        }
        if col > line_len {
            col = line_len;
        }
        if anchor_col > anchor_line_len {
            anchor_col = anchor_line_len;
        }
        return Cursor {
            id: self.id,
            row,
            col,
            anchor_row,
            anchor_col,
        };
    }
}

pub struct Document {
    file_path: String,
    buffer: Buffer,
    cursors: Vec<Cursor>,
}

impl Document {
    pub fn new(file_path: &str) -> Result<Self, io::Error> {
        let mut file = File::open(Path::new(file_path))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let mut cursors = Vec::new();
        cursors.push(Cursor {
            id: 0,
            row: 0,
            col: 0,
            anchor_row: 0,
            anchor_col: 0,
        });

        Ok(Self {
            file_path: file_path.to_string(),
            buffer: Buffer::new(0, BufferId::new(1).unwrap(), contents),
            cursors: cursors,
        })
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
        for cursor in &mut self.cursors {
            let mut cur_x = cursor.col;
            let mut cur_y = cursor.row;
            if cur_x > 0 {
                cur_x -= 1;
            } else {
                if cur_y > 0 {
                    cur_y -= 1;
                    cursor.row = cur_y;
                    let line_len = self.buffer.line_len(cur_y);
                    cur_x = line_len;
                }
            }
            cursor.col = cur_x;
            if !anchor {
                cursor.anchor_col = cursor.col;
                cursor.anchor_row = cursor.row;
            }
        }
    }

    pub fn move_right(&mut self, anchor: bool) {
        for cursor in &mut self.cursors {
            let mut cur_x = cursor.col;
            let mut cur_y = cursor.row;
            let row_count = self.buffer.row_count();
            let line_len = self.buffer.line_len(cur_y);
            cur_x += 1;
            if cur_x > line_len {
                if cur_y < row_count {
                    cur_x = 0;
                    cur_y += 1;
                    cursor.row = cur_y;
                } else {
                    cur_x = line_len;
                }
            }
            cursor.col = cur_x;
            if !anchor {
                cursor.anchor_col = cursor.col;
                cursor.anchor_row = cursor.row;
            }
        }
    }

    pub fn move_up(&mut self, anchor: bool) {
        for cursor in &mut self.cursors {
            let mut cur_x = cursor.col;
            let mut cur_y = cursor.row;
            if cur_y > 0 {
                cur_y -= 1;
            }
            cursor.row = cur_y;
            if !anchor {
                cursor.anchor_col = cursor.col;
                cursor.anchor_row = cursor.row;
            }
        }
    }

    pub fn move_down(&mut self, anchor: bool) {
        for cursor in &mut self.cursors {
            let mut cur_x = cursor.col;
            let mut cur_y = cursor.row;
            cur_y += 1;
            if cur_y > self.buffer.row_count() {
                cur_y = self.buffer.row_count();
            }
            cursor.row = cur_y;
            if !anchor {
                cursor.anchor_col = cursor.col;
                cursor.anchor_row = cursor.row;
            }
        }
    }

    pub fn move_to_next_word() {}
    pub fn move_to_previous_word() {}

    pub fn move_to_start_of_line(&mut self, anchor: bool) {
        for cursor in &mut self.cursors {
            cursor.col = 0;
            if !anchor {
                cursor.anchor_col = cursor.col;
                cursor.anchor_row = cursor.row;
            }
        }
    }

    pub fn move_to_end_of_line(&mut self, anchor: bool) {
        for cursor in &mut self.cursors {
            let line_len = self.buffer.line_len(cursor.row);
            cursor.col = line_len;
            if !anchor {
                cursor.anchor_col = cursor.col;
                cursor.anchor_row = cursor.row;
            }
        }
    }

    pub fn move_to_start_of_document(&mut self, anchor: bool) {
        for cursor in &mut self.cursors {
            cursor.row = 0;
            if !anchor {
                cursor.anchor_col = cursor.col;
                cursor.anchor_row = cursor.row;
            }
        }
    }
    pub fn move_to_end_of_document(&mut self, anchor: bool) {
        for cursor in &mut self.cursors {
            cursor.row = self.buffer.row_count();
            if !anchor {
                cursor.anchor_col = cursor.col;
                cursor.anchor_row = cursor.row;
            }
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        for cursor in &mut self.cursors {
            let cur = cursor.normalized().sane(&self.buffer);
            let start = Point::new(cur.row, cur.col).to_offset(&self.buffer);
            let end = start;
            self.buffer.edit([(start..end, text)]);
        }
    }

    pub fn delete_text(&mut self, count: usize) {
        for cursor in &mut self.cursors {
            let cur = cursor.normalized().sane(&self.buffer);
            let mut start = Point::new(cur.anchor_row, cur.anchor_col).to_offset(&self.buffer);
            let mut end = Point::new(cur.row, cur.col).to_offset(&self.buffer);
            if start == end {
                end += count;
            }
            self.buffer.edit([(start..end, "")]);
            if start != end {
                cursor.row = cur.anchor_row;
                cursor.col = cur.anchor_col;
                cursor.anchor_row = cur.anchor_row;
                cursor.anchor_col = cur.anchor_col;
            }
        }
    }

    pub fn edit(&mut self, start: usize, end: usize, text: &str) -> &Buffer {
        self.buffer.edit([(start..end, text)]);
        &self.buffer
    }

    pub fn cursor(&self, id: usize) -> Cursor {
        return self.cursors[0].clone();
    }
}
