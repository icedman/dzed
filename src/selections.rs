use crate::document::BufferText;
use crate::search::{TextSearch, compile};
use onig::Regex;
use rope::Point;
use std::{cmp::Ordering, ops::Range};
use sum_tree::Bias;
use text::{Anchor, Buffer, Selection, SelectionGoal, ToOffset, ToPoint};

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
pub trait Motions {
    fn move_to_start_of_document(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_end_of_document(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;

    fn move_left_once(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_right_once(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;

    fn move_to_start_of_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_start_of_line_non_space(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_end_of_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_line(&self, anchor: bool, line: u32, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_previous_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_next_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;

    fn find_character(
        &self,
        anchor: bool,
        count: u32,
        ch: char,
        forward: bool,
        buffer: &Buffer,
    ) -> Selection<Anchor>;

    fn move_to_previous_match(&mut self, text: &str, buffer: &Buffer) -> Option<Selection<Anchor>>;
    fn move_to_next_match(&mut self, text: &str, buffer: &Buffer) -> Option<Selection<Anchor>>;
    fn move_to_previous_pattern_match(
        &mut self,
        search: &Regex,
        buffer: &Buffer,
    ) -> Option<Selection<Anchor>>;
    fn move_to_next_pattern_match(
        &mut self,
        search: &Regex,
        buffer: &Buffer,
    ) -> Option<Selection<Anchor>>;
}

impl Motions for Selection<Anchor> {
    fn move_to_start_of_document(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        let point = Point { row: 0, column: 0 };
        let offset = point.to_offset(&buffer);
        let new_head = buffer.anchor_at(offset, Bias::Left);
        return Selection {
            id: self.id,
            start: new_head,
            end: if anchor { self.tail() } else { new_head },
            reversed: true,
            goal: SelectionGoal::None,
        };
    }

    fn move_to_end_of_document(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        let point = Point {
            row: buffer.row_count(),
            column: 0,
        };
        let offset = point.to_offset(&buffer);
        let new_head = buffer.anchor_at(offset, Bias::Left);
        return Selection {
            id: self.id,
            start: new_head,
            end: if anchor { self.tail() } else { new_head },
            reversed: true,
            goal: SelectionGoal::None,
        };
    }

    fn move_left_once(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        let mut point = self.head().to_point(&buffer);
        if point.column != 0 {
            point.column = point.column.saturating_sub(1);
        } else if point.row > 0 {
            point.row = point.row.saturating_sub(1);
            point.column = buffer.line_len(point.row);
        }
        let mut offset = buffer.offset_for_anchor(&buffer.anchor_at(&point, Bias::Left));
        offset = buffer.clip_offset(offset, Bias::Left);
        let new_head = buffer.anchor_at(offset, Bias::Left);
        Selection {
            id: self.id,
            start: new_head,
            end: if anchor { self.tail() } else { new_head },
            reversed: true,
            goal: SelectionGoal::None,
        }
    }

    fn move_right_once(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        let mut point = self.head().to_point(&buffer);
        let l = {
            let row_text = buffer.row_text(point.row);
            row_text.len()
        } as u32;
        if point.column < l {
            point.column += 1;
        } else {
            point.row += 1;
            point.column = 0;
        }
        let mut offset = buffer.offset_for_anchor(&buffer.anchor_at(&point, Bias::Left));
        offset = buffer.clip_offset(offset, Bias::Right);
        let new_head = buffer.anchor_at(offset, Bias::Left);
        Selection {
            id: self.id,
            start: new_head,
            end: if anchor { self.tail() } else { new_head },
            reversed: true,
            goal: SelectionGoal::None,
        }
    }

    fn move_to_start_of_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        let mut point = self.head().to_point(&buffer);
        point.column = 0;
        point = buffer.clip_point(point, self.head().bias);
        let offset = point.to_offset(&buffer);
        let new_head = buffer.anchor_at(offset, Bias::Left);
        Selection {
            id: self.id,
            start: new_head,
            end: if anchor { self.tail() } else { new_head },
            reversed: true,
            goal: SelectionGoal::None,
        }
    }

    fn move_to_start_of_line_non_space(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        let mut point = self.head().to_point(&buffer);
        let line_text = buffer.row_text(point.row);
        let mut first_non_space = 0;
        for (idx, ch) in line_text.char_indices() {
            if !ch.is_whitespace() {
                first_non_space = idx;
                break;
            }
        }
        point.column = first_non_space as u32;
        point = buffer.clip_point(point, self.head().bias);
        let offset = point.to_offset(&buffer);
        let new_head = buffer.anchor_at(offset, Bias::Left);
        Selection {
            id: self.id,
            start: new_head,
            end: if anchor { self.tail() } else { new_head },
            reversed: true,
            goal: SelectionGoal::None,
        }
    }

    fn move_to_end_of_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        let mut point = self.head().to_point(buffer);
        point.column = buffer.line_len(point.row);
        point = buffer.clip_point(point, self.head().bias);
        let offset = point.to_offset(&buffer);
        let new_head = buffer.anchor_at(offset, Bias::Left);
        Selection {
            id: self.id,
            start: new_head,
            end: if anchor { self.tail() } else { new_head },
            reversed: true,
            goal: SelectionGoal::None,
        }
    }

    fn move_to_line(&self, anchor: bool, line: u32, buffer: &Buffer) -> Selection<Anchor> {
        let mut point = self.head().to_point(buffer);
        point.row = line
            .saturating_sub(1)
            .min(buffer.row_count().saturating_sub(1));
        point.column = 0;
        point = buffer.clip_point(point, self.head().bias);
        let offset = point.to_offset(buffer);
        let new_head = buffer.anchor_at(offset, Bias::Left);
        Selection {
            id: self.id,
            start: new_head,
            end: if anchor { self.tail() } else { new_head },
            reversed: false,
            goal: SelectionGoal::None,
        }
    }

    fn move_to_previous_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        let cursor = self.move_to_start_of_line(anchor, buffer);
        cursor.move_left_once(anchor, buffer)
    }

    fn move_to_next_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        let cursor = self.move_to_end_of_line(anchor, buffer);
        cursor.move_right_once(anchor, buffer)
    }

    fn find_character(
        &self,
        anchor: bool,
        count: u32,
        ch: char,
        forward: bool,
        buffer: &Buffer,
    ) -> Selection<Anchor> {
        let mut point = self.head().to_point(buffer);
        let line_text = buffer.row_text(point.row);
        let mut found_count = 0;
        if forward {
            let start_idx = (point.column as usize).saturating_add(1);
            if start_idx < line_text.len() {
                for (idx, c) in line_text[start_idx..].char_indices() {
                    if c == ch {
                        found_count += 1;
                        if found_count == count {
                            point.column = (start_idx + idx) as u32;
                            break;
                        }
                    }
                }
            }
        } else {
            let end_idx = point.column as usize;
            if end_idx > 0 {
                for (idx, c) in line_text[..end_idx].char_indices().rev() {
                    if c == ch {
                        found_count += 1;
                        if found_count == count {
                            point.column = idx as u32;
                            break;
                        }
                    }
                }
            }
        }
        if found_count == count {
            point = buffer.clip_point(point, self.head().bias);
            let offset = point.to_offset(buffer);
            let new_head = buffer.anchor_at(offset, Bias::Left);
            return Selection {
                id: self.id,
                start: new_head,
                end: if anchor { self.tail() } else { new_head },
                reversed: true,
                goal: SelectionGoal::None,
            };
        }
        // not found: return original selection unchanged
        self.clone()
    }

    fn move_to_previous_match(
        &mut self,
        search: &str,
        buffer: &Buffer,
    ) -> Option<Selection<Anchor>> {
        let mut point = self.head().to_point(buffer);
        let line_text = buffer.row_text(point.row);
        if let Some(matched) = line_text
            .to_string()
            .as_str()
            .find_previous_match(search, point.column as usize)
        {
            point.column = matched.0 as u32;
            point = buffer.clip_point(point, self.head().bias);
            let offset = point.to_offset(buffer);
            let new_head = buffer.anchor_at(offset, Bias::Left);
            return Some(Selection {
                id: self.id,
                start: new_head,
                end: new_head,
                reversed: true,
                goal: SelectionGoal::None,
            });
        }
        return None;
    }

    fn move_to_next_match(&mut self, search: &str, buffer: &Buffer) -> Option<Selection<Anchor>> {
        let mut point = self.head().to_point(buffer);
        let line_text = buffer.row_text(point.row);
        if let Some(matched) = line_text
            .to_string()
            .as_str()
            .find_next_match(search, point.column as usize)
        {
            point.column = matched.0 as u32;
            point = buffer.clip_point(point, self.head().bias);
            let offset = point.to_offset(buffer);
            let new_head = buffer.anchor_at(offset, Bias::Left);
            return Some(Selection {
                id: self.id,
                start: new_head,
                end: new_head,
                reversed: true,
                goal: SelectionGoal::None,
            });
        }
        return None;
    }

    fn move_to_previous_pattern_match(
        &mut self,
        search: &Regex,
        buffer: &Buffer,
    ) -> Option<Selection<Anchor>> {
        let mut point = self.head().to_point(buffer);
        let line_text = buffer.row_text(point.row);
        if let Some(matched) = line_text
            .to_string()
            .as_str()
            .find_previous_pattern_match(search, point.column as usize)
        {
            point.column = matched.0 as u32;
            point = buffer.clip_point(point, self.head().bias);
            let offset = point.to_offset(buffer);
            let new_head = buffer.anchor_at(offset, Bias::Left);
            return Some(Selection {
                id: self.id,
                start: new_head,
                end: new_head,
                reversed: true,
                goal: SelectionGoal::None,
            });
        }
        return None;
    }

    fn move_to_next_pattern_match(
        &mut self,
        search: &Regex,
        buffer: &Buffer,
    ) -> Option<Selection<Anchor>> {
        let mut point = self.head().to_point(buffer);
        let line_text = buffer.row_text(point.row);
        if let Some(matched) = line_text
            .to_string()
            .as_str()
            .find_next_pattern_match(search, point.column as usize)
        {
            point.column = matched.0 as u32;
            point = buffer.clip_point(point, self.head().bias);
            let offset = point.to_offset(buffer);
            let new_head = buffer.anchor_at(offset, Bias::Left);
            return Some(Selection {
                id: self.id,
                start: new_head,
                end: new_head,
                reversed: true,
                goal: SelectionGoal::None,
            });
        }
        return None;
    }
}

pub struct SelectionCollection {
    pub id: usize,
    pub selections: Vec<Selection<Anchor>>,
    pub point: Point,
    pub search: String,
    pub regex: Option<Regex>,
}

impl SelectionCollection {
    pub fn new() -> Self {
        return SelectionCollection {
            selections: Vec::<Selection<Anchor>>::new(),
            id: 0,
            point: Point { row: 0, column: 0 },
            search: "".to_string().clone(),
            regex: None,
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

    pub fn update(&mut self, buffer: &Buffer, selection: &Selection<Anchor>) {
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
            let _head_point = cursor_head.to_point(&buffer);
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

    pub fn clear_selections(&mut self, buffer: &Buffer) {
        for cursor in self.selections.clone().iter() {
            self.update(
                buffer,
                &Selection {
                    id: cursor.id,
                    start: cursor.head(),
                    end: cursor.head(),
                    reversed: false,
                    goal: SelectionGoal::None,
                },
            );
        }
    }

    pub fn move_left(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            let cursors = self.selections.clone();
            for cursor in cursors.iter() {
                let next = cursor.clone().move_left_once(anchor, buffer);
                self.point = next.head().to_point(buffer);
                self.update(buffer, &next);
            }
        }
    }

    pub fn move_right(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            let cursors = self.selections.clone();
            for cursor in cursors.iter() {
                let next = cursor.clone().move_right_once(anchor, buffer);
                self.point = next.head().to_point(buffer);
                self.update(buffer, &next);
            }
        }
    }

    pub fn move_up(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
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
                self.update(buffer, &{
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

    pub fn move_down(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
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
                self.update(buffer, &{
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

    pub fn move_to_start_of_line(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let next = cursor.clone().move_to_start_of_line(anchor, buffer);
            self.update(buffer, &next);
        }
    }

    pub fn move_to_start_of_line_non_space(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let next = cursor
                .clone()
                .move_to_start_of_line_non_space(anchor, buffer);
            self.update(buffer, &next);
        }
    }

    pub fn move_to_end_of_line(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let next = cursor.clone().move_to_end_of_line(anchor, buffer);
            self.update(buffer, &next);
        }
    }

    pub fn move_to_line(&mut self, anchor: bool, line: u32, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let next = cursor.clone().move_to_line(anchor, line, buffer);
            self.update(buffer, &next);
        }
    }

    pub fn find_character(
        &mut self,
        anchor: bool,
        count: u32,
        ch: char,
        forward: bool,
        buffer: &Buffer,
    ) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let next = cursor
                .clone()
                .find_character(anchor, count, ch, forward, buffer);
            // If not found, `next` equals original selection; update anyway is harmless
            self.update(buffer, &next);
        }
    }

    pub fn move_to_start_of_document(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            self.update(
                buffer,
                &cursor.clone().move_to_start_of_document(anchor, buffer),
            );
        }
    }

    pub fn move_to_end_of_document(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            self.update(
                buffer,
                &cursor.clone().move_to_end_of_document(anchor, buffer),
            );
        }
    }

    pub fn move_to_previous_match(&mut self, text: &str, pattern: bool, buffer: &Buffer) {
        if pattern && text != self.search {
            self.search = text.to_string();
            self.regex = compile(self.search.as_str());
        }
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let mut cur = cursor.clone();
            let point = cursor.head().to_point(&buffer);
            if pattern {
                if let Some(ref regex) = self.regex {
                    for _ in 0..(point.row + 1) {
                        if let Some(matched) = cur.move_to_previous_pattern_match(regex, buffer) {
                            self.update(buffer, &matched);
                            break;
                        } else {
                            cur = cur.move_to_previous_line(false, buffer);
                        }
                    }
                }
            } else {
                for _ in 0..(point.row + 1) {
                    if let Some(matched) = cur.move_to_previous_match(text, buffer) {
                        self.update(buffer, &matched);
                        break;
                    } else {
                        cur = cur.move_to_previous_line(false, buffer);
                    }
                }
            }
        }
    }

    pub fn move_to_next_match(&mut self, text: &str, pattern: bool, buffer: &Buffer) {
        if pattern && text != self.search {
            self.search = text.to_string();
            self.regex = compile(self.search.as_str());
        }
        let cursors = self.selections.clone();
        let rows = buffer.row_count();
        for cursor in cursors.iter() {
            let mut cur = cursor.clone();
            let point = cursor.head().to_point(&buffer);

            if pattern {
                if let Some(ref regex) = self.regex {
                    for _ in point.row..rows {
                        if let Some(matched) = cur.move_to_next_pattern_match(regex, buffer) {
                            self.update(buffer, &matched);
                            break;
                        } else {
                            cur = cur.move_to_next_line(false, buffer);
                        }
                    }
                }
            } else {
                for _ in point.row..rows {
                    if let Some(matched) = cur.move_to_next_match(text, buffer) {
                        self.update(buffer, &matched);
                        break;
                    } else {
                        cur = cur.move_to_next_line(false, buffer);
                    }
                }
            }
        }
    }
}
