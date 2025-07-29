use crate::document::BufferText;
use rope::Point;
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
    pub point: Point,
}

impl SelectionCollection {
    pub fn new() -> Self {
        return SelectionCollection {
            selections: Vec::<Selection<Anchor>>::new(),
            id: 0,
            point: Point { row: 0, column: 0 },
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

    pub fn is_selected(&self, row: u32, column: u32, buffer: &Buffer) -> (bool, bool, bool) {
        let mut within = true;
        let mut within_line = true;
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
                within_line = false;
                at_head = false;
                continue;
            }
            if row == start.row {
                let sc = start.column;
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
                let ec = end.column;
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
        (within, within_line, at_head)
    }

    pub fn has_selection(&self, buffer: &Buffer) -> bool {
        for cursor in self.selections.iter() {
            if cursor.head().cmp(&cursor.tail(), &buffer) != Ordering::Equal {
                return true;
            }
        }
        return false;
    }

    pub fn clear_selections(&mut self) {
        for cursor in self.selections.clone().iter() {
            self.update(&Selection {
                id: cursor.id,
                start: cursor.head(),
                end: cursor.tail(),
                reversed: false,
                goal: SelectionGoal::None,
            });
        }
    }

    pub fn move_left(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let mut point = cursor.head().to_point(&buffer);
            if point.column != 0 {
                point.column = point.column.saturating_sub(1);
            } else {
                if point.row > 0 {
                    point.row = point.row.saturating_sub(1);
                    point.column = buffer.line_len(point.row);
                }
            };
            self.point = point;
            let mut offset = buffer.offset_for_anchor(&buffer.anchor_at(&point, Bias::Left));
            offset = buffer.clip_offset(offset, Bias::Left);
            let new_head = buffer.anchor_at(offset, Bias::Left);
            self.update(&{
                Selection {
                    id: cursor.id,
                    start: new_head,
                    end: if anchor { cursor.tail() } else { new_head },
                    reversed: true,
                    goal: SelectionGoal::None,
                }
            });
        }
    }

    pub fn move_right(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let mut point = cursor.head().to_point(&buffer);
            let l = {
                let row_text = buffer.row_text(point.row);
                row_text.len()
            };
            if point.column < l as u32 {
                point.column += 1;
            } else {
                point.row += 1;
                point.column = 0;
            };
            self.point = point;
            let mut offset = buffer.offset_for_anchor(&buffer.anchor_at(&point, Bias::Left));
            offset = buffer.clip_offset(offset, Bias::Right);
            let new_head = buffer.anchor_at(offset, Bias::Left);
            self.update(&{
                Selection {
                    id: cursor.id,
                    start: new_head,
                    end: if anchor { cursor.tail() } else { new_head },
                    reversed: true,
                    goal: SelectionGoal::None,
                }
            });
        }
    }

    pub fn move_up(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let mut point = cursor.head().to_point(&buffer);
            point.row = point.row.saturating_sub(1);
            if self.point.column < buffer.line_len(point.row) {
                point.column = self.point.column;
            }
            point = buffer.clip_point(point, cursor.head().bias);
            let offset = point.to_offset(&buffer);
            let new_head = buffer.anchor_at(offset, Bias::Left);
            self.update(&{
                Selection {
                    id: cursor.id,
                    start: new_head,
                    end: if anchor { cursor.tail() } else { new_head },
                    reversed: true,
                    goal: SelectionGoal::None,
                }
            });
        }
    }

    pub fn move_down(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let mut point = cursor.head().to_point(&buffer);
            point.row += 1;
            if self.point.column < buffer.line_len(point.row) {
                point.column = self.point.column;
            }
            point = buffer.clip_point(point, cursor.head().bias);
            let offset = point.to_offset(&buffer);
            let new_head = buffer.anchor_at(offset, Bias::Left);
            self.update(&{
                Selection {
                    id: cursor.id,
                    start: new_head,
                    end: if anchor { cursor.tail() } else { new_head },
                    reversed: true,
                    goal: SelectionGoal::None,
                }
            });
        }
    }

    pub fn move_up_count(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            self.move_up(anchor, &buffer);
        }
    }

    pub fn move_down_count(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            self.move_down(anchor, &buffer);
        }
    }

    pub fn move_to_start_of_line(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let mut point = cursor.head().to_point(&buffer);
            point.column = 0;
            point = buffer.clip_point(point, cursor.head().bias);
            let offset = point.to_offset(&buffer);
            let new_head = buffer.anchor_at(offset, Bias::Left);
            self.update(&{
                Selection {
                    id: cursor.id,
                    start: new_head,
                    end: if anchor { cursor.tail() } else { new_head },
                    reversed: true,
                    goal: SelectionGoal::None,
                }
            });
        }
    }

    pub fn move_to_end_of_line(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let mut point = cursor.head().to_point(buffer);
            point.column = buffer.line_len(point.row);
            point = buffer.clip_point(point, cursor.head().bias);
            let offset = point.to_offset(&buffer);
            let new_head = buffer.anchor_at(offset, Bias::Left);
            self.update(&{
                Selection {
                    id: cursor.id,
                    start: new_head,
                    end: if anchor { cursor.tail() } else { new_head },
                    reversed: true,
                    goal: SelectionGoal::None,
                }
            });
        }
    }

    pub fn move_to_start_of_document(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let point = Point { row: 0, column: 0 };
            let offset = point.to_offset(&buffer);
            let new_head = buffer.anchor_at(offset, Bias::Left);
            self.update(&{
                Selection {
                    id: cursor.id,
                    start: new_head,
                    end: if anchor { cursor.tail() } else { new_head },
                    reversed: true,
                    goal: SelectionGoal::None,
                }
            });
        }
    }

    pub fn move_to_end_of_document(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let point = Point {
                row: buffer.row_count(),
                column: 0,
            };
            let offset = point.to_offset(&buffer);
            let new_head = buffer.anchor_at(offset, Bias::Left);
            self.update(&{
                Selection {
                    id: cursor.id,
                    start: new_head,
                    end: if anchor { cursor.tail() } else { new_head },
                    reversed: true,
                    goal: SelectionGoal::None,
                }
            });
        }
    }
}
