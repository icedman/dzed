use crate::document::BufferText;
use std::{cmp::Ordering, ops::Range};
use text::{Anchor, AnchorRangeExt, Buffer, Selection, SelectionGoal, ToOffset, ToPoint};

use sum_tree::Bias;

pub fn offset_to_column(text: &String, offset: usize) -> u32 {
    let mut cc = 0;
    for (i, c) in text.chars().enumerate() {
        if cc == offset {
            return i as u32;
        }
        cc += c.len_utf8();
    }
    0
}

pub struct SelectionCollection {
    pub id: usize,
    pub selections: Vec<Selection<Anchor>>,
}

impl SelectionCollection {
    pub fn new() -> Self {
        return SelectionCollection {
            selections: Vec::<Selection<Anchor>>::new(),
            id: 0,
        };
    }

    pub fn first(&self) -> Option<&Selection<Anchor>> {
        self.selections.first()
    }

    pub fn last(&self) -> Option<&Selection<Anchor>> {
        self.selections.last()
    }

    pub fn add(&mut self, buffer: &Buffer, offset: usize) -> Selection<Anchor> {
        let sel = Selection {
            id: self.id,
            start: buffer.anchor_at(offset, Bias::Left),
            end: buffer.anchor_at(offset, Bias::Left),
            reversed: false,
            goal: SelectionGoal::None,
        };
        self.selections.push(sel.clone());
        self.id += 1;
        sel
    }

    pub fn update(&mut self, selection: &Selection<Anchor>) {
        if let Some(selected) = self.selections.iter_mut().find(|s| s.id == selection.id) {
            *selected = selection.clone();
        }
    }

    pub fn clear(&mut self) {
        self.selections.clear();
    }

    pub fn is_selected(&self, row: u32, column: u32, buffer: &Buffer) -> (bool, bool) {
        let mut within = true;
        let mut at_head = false;
        for cursor in self.selections.iter() {
            let cursor_head = cursor.head();
            let head_point = cursor_head.to_point(&buffer);
            let cursor_tail = cursor.tail();
            let (cursor_range, normalized) =
                if cursor_head.cmp(&cursor_tail, &buffer) == Ordering::Less {
                    (
                        Range {
                            start: cursor_head,
                            end: cursor_tail,
                        },
                        false,
                    )
                } else {
                    (
                        Range {
                            end: cursor_head,
                            start: cursor_tail,
                        },
                        true,
                    )
                };

            let start = cursor_range.start.to_point(&buffer);
            let end = cursor_range.end.to_point(&buffer);
            if row < start.row || row > end.row {
                within = false;
                at_head = false;
                continue;
            }
            if row == start.row {
                let st = buffer.row_text(row);
                let sc = offset_to_column(&st, start.column as usize);
                if column < sc {
                    within = false;
                    at_head = false;
                    continue;
                }
                if !normalized && column == sc {
                    at_head = true;
                }
            }
            if row == end.row {
                let st = buffer.row_text(row);
                let ec = offset_to_column(&st, end.column as usize);
                if column > ec {
                    within = false;
                    at_head = false;
                    continue;
                }
                if normalized && column == ec {
                    at_head = true;
                }
            }
            if within && at_head {
                break;
            }
        }
        (within, at_head)
    }
}
