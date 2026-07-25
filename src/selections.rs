use crate::document::BufferText;
use crate::search::{compile, TextSearch};
use onig::Regex;
use rope::Point;
use std::cmp::Ordering;
use sum_tree::Bias;
use text::{Anchor, Buffer, Selection, SelectionGoal, ToOffset, ToPoint};

pub trait Motions {
    fn text(&self, buffer: &Buffer) -> String;

    fn move_to_start_of_document(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_end_of_document(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;

    fn move_left_once(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_right_once(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_up_once(&self, anchor: bool, column: u32, buffer: &Buffer) -> Selection<Anchor>;
    fn move_down_once(&self, anchor: bool, column: u32, buffer: &Buffer) -> Selection<Anchor>;

    fn move_to_start_of_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_start_of_line_non_space(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_end_of_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_line(&self, anchor: bool, line: u32, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_previous_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_next_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_start_of_previous_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_end_of_previous_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_start_of_next_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_end_of_next_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;

    fn find_character(
        &self,
        anchor: bool,
        count: u32,
        ch: char,
        forward: bool,
        till: bool,
        buffer: &Buffer,
    ) -> Selection<Anchor>;

    // Word motions
    fn move_to_word(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_word_end(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_previous_word(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_next_word(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_next_word_end(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_previous_word_end(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_big_word(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_previous_big_word(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_big_word_end(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_previous_big_word_end(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;

    // Paragraph motions
    fn move_to_previous_paragraph(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;
    fn move_to_next_paragraph(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor>;

    fn move_to_previous_match(&mut self, text: &str, buffer: &Buffer) -> Option<Selection<Anchor>>;
    fn move_to_next_match(&mut self, text: &str, buffer: &Buffer) -> Option<Selection<Anchor>>;
    fn move_to_next_match_within(
        &mut self,
        search: &str,
        buffer: &Buffer,
        rows: u32,
    ) -> Option<Selection<Anchor>>;
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

    fn move_to_syntax_target<F>(
        &self,
        anchor: bool,
        syntax_tree: &crate::treesitter::SyntaxTree,
        buffer: &Buffer,
        target: F,
    ) -> Option<Selection<Anchor>>
    where
        F: Fn(&crate::treesitter::SyntaxTree, usize) -> Option<crate::treesitter::SyntaxNode>;

    fn move_to_syntax_target_end<F>(
        &self,
        anchor: bool,
        syntax_tree: &crate::treesitter::SyntaxTree,
        buffer: &Buffer,
        target: F,
    ) -> Option<Selection<Anchor>>
    where
        F: Fn(&crate::treesitter::SyntaxTree, usize) -> Option<crate::treesitter::SyntaxNode>;
}

impl Motions for Selection<Anchor> {
    fn text(&self, buffer: &Buffer) -> String {
        let head = self.head();
        let tail = self.tail();
        if head.cmp(&tail, buffer) == Ordering::Equal {
            return String::new();
        }

        let (start, end) = if head.cmp(&tail, buffer) == Ordering::Less {
            (head.bias_left(buffer), tail.bias_right(buffer))
        } else {
            (tail.bias_left(buffer), head.bias_right(buffer))
        };
        let start = buffer.offset_for_anchor(&start);
        let end = buffer.clip_offset(buffer.offset_for_anchor(&end) + 1, Bias::Right);

        buffer.as_rope().chunks_in_range(start..end).collect()
    }

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
            let row_text = buffer.row_text(point.row);
            let current_col = point.column as usize;
            if let Some(ch) = row_text[..current_col].chars().next_back() {
                point.column = point.column.saturating_sub(ch.len_utf8() as u32);
            } else {
                point.column = point.column.saturating_sub(1);
            }
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
        let row_text = buffer.row_text(point.row);
        let l = row_text.len() as u32;
        if point.column < l {
            let current_col = point.column as usize;
            if let Some(ch) = row_text[current_col..].chars().next() {
                point.column += ch.len_utf8() as u32;
            } else {
                point.column += 1;
            }
        } else {
            point.row += 1;
            point.column = 0;
        }
        let mut offset = buffer.offset_for_anchor(&buffer.anchor_at(&point, Bias::Right));
        offset = buffer.clip_offset(offset, Bias::Right);
        let new_head = buffer.anchor_at(offset, Bias::Right);
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

    fn move_up_once(&self, anchor: bool, column: u32, buffer: &Buffer) -> Selection<Anchor> {
        let mut point = self.head().to_point(buffer);
        point.row = point.row.saturating_sub(1);
        point.column = column.min(buffer.line_len(point.row));
        point = buffer.clip_point(point, self.head().bias);
        let new_head = buffer.anchor_at(point.to_offset(buffer), Bias::Left);

        Selection {
            id: self.id,
            start: new_head,
            end: if anchor { self.tail() } else { new_head },
            reversed: true,
            goal: SelectionGoal::None,
        }
    }

    fn move_down_once(&self, anchor: bool, column: u32, buffer: &Buffer) -> Selection<Anchor> {
        let mut point = self.head().to_point(buffer);
        point.row = point
            .row
            .saturating_add(1)
            .min(buffer.row_count().saturating_sub(1));
        point.column = column.min(buffer.line_len(point.row));
        point = buffer.clip_point(point, self.head().bias);
        let new_head = buffer.anchor_at(point.to_offset(buffer), Bias::Left);

        Selection {
            id: self.id,
            start: new_head,
            end: if anchor { self.tail() } else { new_head },
            reversed: true,
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

    fn move_to_start_of_previous_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        let cursor = self.move_to_previous_line(anchor, buffer);
        cursor.move_to_start_of_line(anchor, buffer)
    }

    fn move_to_end_of_previous_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        self.move_to_previous_line(anchor, buffer)
    }

    fn move_to_start_of_next_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        self.move_to_next_line(anchor, buffer)
    }

    fn move_to_end_of_next_line(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        let cursor = self.move_to_next_line(anchor, buffer);
        cursor.move_to_end_of_line(anchor, buffer)
    }

    fn find_character(
        &self,
        anchor: bool,
        count: u32,
        ch: char,
        forward: bool,
        till: bool,
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
                            let match_idx = start_idx + idx;
                            if till {
                                if let Some(prev_c) = line_text[..match_idx].chars().next_back() {
                                    point.column = (match_idx - prev_c.len_utf8()) as u32;
                                } else {
                                    point.column = match_idx as u32;
                                }
                            } else {
                                point.column = match_idx as u32;
                            }
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
                            if till {
                                point.column = (idx + c.len_utf8()) as u32;
                            } else {
                                point.column = idx as u32;
                            }
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

    fn move_to_word(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        use crate::search::TextSearch;
        let mut point = self.head().to_point(buffer);
        let text = buffer.row_text(point.row);
        let _previous_column = point.column;
        if let Some(word) = text.as_str().find_word(point.column as usize) {
            point.column = word.0 as u32;
        } else {
            point.column = 0;
        }
        let offset = point.to_offset(buffer);
        let new_head = buffer.anchor_at(offset, Bias::Left);
        Selection {
            id: self.id,
            start: new_head,
            end: if anchor { self.tail() } else { new_head },
            reversed: true,
            goal: SelectionGoal::None,
        }
    }

    fn move_to_word_end(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        use crate::search::TextSearch;
        let mut point = self.head().to_point(buffer);
        let text = buffer.row_text(point.row);
        if let Some(word) = text.as_str().find_word(point.column as usize) {
            point.column = (word.1 - 1) as u32;
        } else {
            point.column = buffer.line_len(point.row);
        }
        let mut offset = point.to_offset(buffer);
        offset = buffer.clip_offset(offset, Bias::Left);
        let new_head = buffer.anchor_at(offset, Bias::Left);
        Selection {
            id: self.id,
            end: new_head,
            start: if anchor { self.tail() } else { new_head },
            reversed: false,
            goal: SelectionGoal::None,
        }
    }

    fn move_to_previous_word(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        use crate::search::TextSearch;
        let mut point = self.head().to_point(buffer);
        let text = buffer.row_text(point.row);
        let previous_column = point.column;
        if let Some(word) = text.as_str().find_previous_word(point.column as usize) {
            point.column = word.0 as u32;
        } else {
            point.column = 0;
        }
        if point.column == previous_column {
            return self.move_left_once(anchor, buffer);
        }
        let offset = point.to_offset(buffer);
        let new_head = buffer.anchor_at(offset, Bias::Left);
        Selection {
            id: self.id,
            start: new_head,
            end: if anchor { self.tail() } else { new_head },
            reversed: true,
            goal: SelectionGoal::None,
        }
    }

    fn move_to_next_word(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        use crate::search::TextSearch;
        let mut point = self.head().to_point(buffer);
        let text = buffer.row_text(point.row);
        let previous_column = point.column;
        if let Some(word) = text.as_str().find_next_word(point.column as usize) {
            point.column = word.0 as u32;
        } else {
            point.column = buffer.line_len(point.row);
        }
        if point.column == previous_column {
            return self.move_right_once(anchor, buffer);
        }
        let mut offset = point.to_offset(buffer);
        offset = buffer.clip_offset(offset, Bias::Left);
        let new_head = buffer.anchor_at(offset, Bias::Right);
        Selection {
            id: self.id,
            end: new_head,
            start: if anchor { self.tail() } else { new_head },
            reversed: false,
            goal: SelectionGoal::None,
        }
    }

    fn move_to_next_word_end(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        use crate::search::TextSearch;
        let mut point = self.head().to_point(buffer);
        let text = buffer.row_text(point.row);
        if let Some(word) = text.as_str().find_next_word_end(point.column as usize) {
            point.column = (word.1 - 1) as u32;
        } else {
            point.column = buffer.line_len(point.row);
        }
        let mut offset = point.to_offset(buffer);
        offset = buffer.clip_offset(offset, Bias::Left);
        let new_head = buffer.anchor_at(offset, Bias::Left);
        Selection {
            id: self.id,
            end: new_head,
            start: if anchor { self.tail() } else { new_head },
            reversed: false,
            goal: SelectionGoal::None,
        }
    }

    fn move_to_previous_word_end(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        use crate::search::TextSearch;
        let mut point = self.head().to_point(buffer);
        let text = buffer.row_text(point.row);
        if let Some(word) = text.as_str().find_previous_word_end(point.column as usize) {
            point.column = (word.1 - 1) as u32;
        } else {
            point.column = 0;
        }
        let offset = point.to_offset(buffer);
        let new_head = buffer.anchor_at(offset, Bias::Left);
        Selection {
            id: self.id,
            start: new_head,
            end: if anchor { self.tail() } else { new_head },
            reversed: true,
            goal: SelectionGoal::None,
        }
    }

    fn move_to_big_word(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        use crate::search::TextSearch;
        let mut point = self.head().to_point(buffer);
        let text = buffer.row_text(point.row);
        if let Some(word) = text.as_str().find_next_big_word(point.column as usize) {
            point.column = word.0 as u32;
        } else {
            point.column = text.len() as u32;
        }
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

    fn move_to_previous_big_word(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        use crate::search::TextSearch;
        let mut point = self.head().to_point(buffer);
        let text = buffer.row_text(point.row);
        if let Some(word) = text.as_str().find_previous_big_word(point.column as usize) {
            point.column = word.0 as u32;
        } else {
            point.column = 0;
        }
        let offset = point.to_offset(buffer);
        let new_head = buffer.anchor_at(offset, Bias::Left);
        Selection {
            id: self.id,
            start: new_head,
            end: if anchor { self.tail() } else { new_head },
            reversed: true,
            goal: SelectionGoal::None,
        }
    }

    fn move_to_big_word_end(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        use crate::search::TextSearch;
        let mut point = self.head().to_point(buffer);
        let text = buffer.row_text(point.row);
        if let Some(word) = text.as_str().find_next_big_word_end(point.column as usize) {
            point.column = (word.1 - 1) as u32;
        } else {
            point.column = text.len() as u32;
        }
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

    fn move_to_previous_big_word_end(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        use crate::search::TextSearch;
        let mut point = self.head().to_point(buffer);
        let text = buffer.row_text(point.row);
        if let Some(word) = text.as_str().find_previous_big_word_end(point.column as usize) {
            point.column = (word.1 - 1) as u32;
        } else {
            point.column = 0;
        }
        let offset = point.to_offset(buffer);
        let new_head = buffer.anchor_at(offset, Bias::Left);
        Selection {
            id: self.id,
            start: new_head,
            end: if anchor { self.tail() } else { new_head },
            reversed: true,
            goal: SelectionGoal::None,
        }
    }

    fn move_to_previous_paragraph(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        let mut point = self.head().to_point(buffer);
        point.column = 0;
        let mut target_point = point.clone();
        let mut has_target = false;
        while point.row > 0 {
            point.row -= 1;
            if buffer.line_len(point.row) == 0 {
                target_point = point.clone();
                has_target = true;
            } else if has_target {
                break;
            }
        }
        let final_point = if has_target { target_point } else { point };
        let mut offset = final_point.to_offset(buffer);
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

    fn move_to_next_paragraph(&self, anchor: bool, buffer: &Buffer) -> Selection<Anchor> {
        let mut point = self.head().to_point(buffer);
        point.column = 0;
        let mut target_point = point.clone();
        let mut has_target = false;
        while point.row < buffer.row_count() {
            point.row += 1;
            if buffer.line_len(point.row) == 0 {
                target_point = point.clone();
                has_target = true;
            } else if has_target {
                break;
            }
        }
        let final_point = if has_target { target_point } else { point };
        let offset = final_point.to_offset(buffer);
        let new_head = buffer.anchor_at(offset, Bias::Left);
        Selection {
            id: self.id,
            start: new_head,
            end: if anchor { self.tail() } else { new_head },
            reversed: true,
            goal: SelectionGoal::None,
        }
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

    fn move_to_next_match_within(
        &mut self,
        search: &str,
        buffer: &Buffer,
        rows: u32,
    ) -> Option<Selection<Anchor>> {
        let mut cursor = self.clone();
        if let Some(matched) = cursor.move_to_next_match(search, buffer) {
            return Some(matched);
        }

        for _ in 0..rows {
            let current_row = cursor.head().to_point(buffer).row;
            if current_row + 1 >= buffer.row_count() {
                cursor = cursor.move_to_start_of_document(false, buffer);
            } else {
                cursor = cursor.move_to_start_of_next_line(false, buffer);
            }

            let mut point = cursor.head().to_point(buffer);
            let line_text = buffer.row_text(point.row);
            let Some((column, _, _)) = line_text.find_string(search).into_iter().next() else {
                continue;
            };

            point.column = column as u32;
            point = buffer.clip_point(point, cursor.head().bias);
            let new_head = buffer.anchor_at(point.to_offset(buffer), Bias::Left);
            return Some(Selection {
                id: cursor.id,
                start: new_head,
                end: new_head,
                reversed: true,
                goal: SelectionGoal::None,
            });
        }

        None
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

    fn move_to_syntax_target<F>(
        &self,
        anchor: bool,
        syntax_tree: &crate::treesitter::SyntaxTree,
        buffer: &Buffer,
        target: F,
    ) -> Option<Selection<Anchor>>
    where
        F: Fn(&crate::treesitter::SyntaxTree, usize) -> Option<crate::treesitter::SyntaxNode>,
    {
        let byte = buffer.offset_for_anchor(&self.head());
        let Some(node) = target(syntax_tree, byte) else {
            return None;
        };
        let head = buffer.anchor_at(node.byte_range.start, Bias::Left);
        Some(Selection {
            id: self.id,
            start: head,
            end: if anchor { self.tail() } else { head },
            reversed: false,
            goal: SelectionGoal::None,
        })
    }

    fn move_to_syntax_target_end<F>(
        &self,
        anchor: bool,
        syntax_tree: &crate::treesitter::SyntaxTree,
        buffer: &Buffer,
        target: F,
    ) -> Option<Selection<Anchor>>
    where
        F: Fn(&crate::treesitter::SyntaxTree, usize) -> Option<crate::treesitter::SyntaxNode>,
    {
        let byte = buffer.offset_for_anchor(&self.head());
        let Some(node) = target(syntax_tree, byte) else {
            return None;
        };
        let head = buffer.anchor_at(node.byte_range.end.saturating_sub(1), Bias::Right);
        Some(Selection {
            id: self.id,
            start: head,
            end: if anchor { self.tail() } else { head },
            reversed: false,
            goal: SelectionGoal::None,
        })
    }
}

pub struct SelectionCollection {
    pub id: usize,
    pub selections: Vec<Selection<Anchor>>,
    pub point: Point,
    pub search: String,
    pub regex: Option<Regex>,
    pub anchor: Option<Selection<Anchor>>,
}

impl SelectionCollection {
    pub fn new() -> Self {
        return SelectionCollection {
            selections: Vec::<Selection<Anchor>>::new(),
            id: 0,
            point: Point { row: 0, column: 0 },
            search: "".to_string().clone(),
            regex: None,
            anchor: None,
        };
    }

    pub fn first(&self) -> Option<&Selection<Anchor>> {
        self.selections.first()
    }

    pub fn last(&self) -> Option<&Selection<Anchor>> {
        self.selections.last()
    }

    pub fn has_similar_cursor(&self, cursor: &Selection<Anchor>, buffer: &Buffer) -> bool {
        let head = buffer.offset_for_anchor(&cursor.head());
        let tail = buffer.offset_for_anchor(&cursor.tail());

        self.selections.iter().any(|existing| {
            let existing_head = buffer.offset_for_anchor(&existing.head());
            let existing_tail = buffer.offset_for_anchor(&existing.tail());

            (existing_head == head && existing_tail == tail)
                || (existing_head == tail && existing_tail == head)
        })
    }

    pub fn text(&self, buffer: &Buffer) -> String {
        self.selections
            .iter()
            .map(|selection| selection.text(buffer))
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn rows_in_selection(&self, buffer: &Buffer) -> (u32, u32) {
        let mut start: u32 = buffer.row_count();
        let mut end: u32 = 0;
        for cursor in self.selections.iter() {
            let mut rows = [
                cursor.start.to_point(buffer).row,
                cursor.end.to_point(buffer).row,
            ];
            rows.sort();
            let row_start = rows[0];
            let row_end = rows[1];
            start = std::cmp::min(row_start, start);
            end = std::cmp::max(row_end, end);
        }

        return (start, end);
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

    pub fn update(&mut self, _buffer: &Buffer, selection: &Selection<Anchor>) {
        if let Some(selected) = self.selections.iter_mut().find(|s| s.id == selection.id) {
            *selected = selection.clone();
        }
    }

    pub fn begin_block(&mut self, buffer: &Buffer) {
        if let Some(first) = self.first().cloned() {
            self.anchor = Some(first);
            self.sync_block(buffer);
        }
    }

    pub fn sync_block(&mut self, buffer: &Buffer) {
        if self.selections.is_empty() {
            return;
        }

        let Some(anchor_sel) = self.anchor.clone() else {
            return;
        };
        let first_sel = self.selections[0].clone();

        // Compute row and column bounds from both selections' heads and tails
        let mut rows = [
            anchor_sel.start.to_point(buffer).row,
            anchor_sel.end.to_point(buffer).row,
            first_sel.start.to_point(buffer).row,
            first_sel.end.to_point(buffer).row,
        ];
        rows.sort();
        let row_start = rows[0];
        let row_end = rows[3];

        let mut cols = [
            anchor_sel.start.to_point(buffer).column,
            anchor_sel.end.to_point(buffer).column,
            first_sel.start.to_point(buffer).column,
            first_sel.end.to_point(buffer).column,
        ];
        cols.sort();
        let col_start = cols[0];
        let col_end = cols[3];

        let first_id = first_sel.id;
        let first_row = first_sel.head().to_point(buffer).row;

        // Remove selections that are outside the block row range, except the first selection
        self.selections.retain(|sel| {
            if sel.id == first_id {
                return true;
            }
            let row = sel.head().to_point(buffer).row;
            row >= row_start && row <= row_end
        });

        // Ensure a selection exists on each row within the range (inclusive), except the first row
        for row in row_start..=row_end {
            if row == first_row {
                continue;
            }

            // Find an existing selection on this row (not the first)
            let existing_idx = self
                .selections
                .iter()
                .position(|s| s.id != first_id && s.head().to_point(buffer).row == row);

            let line_len = buffer.line_len(row);
            let s_col = col_start.min(line_len);
            let e_col = col_end.min(line_len);

            let start_pt = Point { row, column: s_col };
            let end_pt = Point { row, column: e_col };
            let start_anchor = buffer.anchor_at(start_pt.to_offset(buffer), Bias::Left);
            let end_anchor = buffer.anchor_at(end_pt.to_offset(buffer), Bias::Left);

            if let Some(idx) = existing_idx {
                let id = self.selections[idx].id;
                self.selections[idx] = Selection {
                    id,
                    start: start_anchor,
                    end: end_anchor,
                    reversed: false,
                    goal: SelectionGoal::None,
                };
            } else {
                let id = self.id;
                self.id += 1;
                self.selections.push(Selection {
                    id,
                    start: start_anchor,
                    end: end_anchor,
                    reversed: false,
                    goal: SelectionGoal::None,
                });
            }
        }

        // Finally, update the first selection so it conforms to the block at its row
        let line_len = buffer.line_len(first_row);
        let s_col = col_start.min(line_len);
        let e_col = col_end.min(line_len);
        let start_pt = Point {
            row: first_row,
            column: s_col,
        };
        let end_pt = Point {
            row: first_row,
            column: e_col,
        };
        let start_anchor = buffer.anchor_at(start_pt.to_offset(buffer), Bias::Left);
        let end_anchor = buffer.anchor_at(end_pt.to_offset(buffer), Bias::Left);
        self.selections[0] = Selection {
            id: first_id,
            start: start_anchor,
            end: end_anchor,
            reversed: false,
            goal: SelectionGoal::None,
        };
    }

    pub fn end_block(&mut self) {
        self.anchor = None;
    }

    pub fn begin_line(&mut self, buffer: &Buffer) {
        self.clear(buffer);
        if let Some(first) = self.first().cloned() {
            self.anchor = Some(first);
            self.sync_line(buffer);
        }
    }

    pub fn sync_line(&mut self, buffer: &Buffer) {
        let Some(current) = self.first().cloned() else {
            return;
        };
        let Some(anchor) = self.anchor.as_ref() else {
            return;
        };

        let head = current.head().to_point(buffer);
        let tail = anchor.head().to_point(buffer);
        let upper_row = head.row.min(tail.row);
        let lower_row = head.row.max(tail.row);

        let upper = Point {
            row: upper_row,
            column: 0,
        };
        let lower = Point {
            row: lower_row,
            column: buffer.line_len(lower_row),
        };
        let upper_anchor = buffer.anchor_at(upper.to_offset(buffer), Bias::Left);
        let lower_anchor = buffer.anchor_at(lower.to_offset(buffer), Bias::Left);

        // Keep the endpoint on the moving cursor's row as the head.
        let reversed = head.row < tail.row;
        self.selections.truncate(1);
        self.selections[0] = Selection {
            id: current.id,
            start: upper_anchor,
            end: lower_anchor,
            reversed,
            goal: SelectionGoal::None,
        };
    }

    pub fn end_line(&mut self) {
        self.anchor = None;
    }

    pub fn clear(&mut self, buffer: &Buffer) {
        self.clear_selections(buffer);
        if let Some(first) = self.first().cloned() {
            self.selections.clear();
            self.selections.push(first);
        }
    }

    pub fn is_selected(&self, row: u32, column: u32, buffer: &Buffer) -> (bool, bool, bool) {
        // Returns (selected_cell, selected_line, at_cursor_head)
        for cursor in self.selections.iter() {
            let head = cursor.head();
            let tail = cursor.tail();
            let (start, end, normalized) = if head.cmp(&tail, buffer) == Ordering::Less {
                (head.to_point(buffer), tail.to_point(buffer), false)
            } else {
                (tail.to_point(buffer), head.to_point(buffer), true)
            };

            // If row is outside this selection's vertical bounds, try next selection
            if row < start.row || row > end.row {
                continue;
            }

            // Row is within selection's vertical range
            let mut selected = true;
            // Horizontal bounds depending on whether we're on boundary rows
            if start.row == end.row {
                // Single-line selection
                selected = column >= start.column && column <= end.column;
            } else if row == start.row {
                selected = column >= start.column;
            } else if row == end.row {
                selected = column <= end.column;
            } else {
                // Middle rows: all columns inside are selected for VisualLine; for VisualBlock, each row
                // has its own start/end via separate selections, so this path is fine as 'selected = true'
                selected = true;
            }

            if selected {
                let at_head = if normalized {
                    row == end.row && column == end.column
                } else {
                    row == start.row && column == start.column
                };
                let selected_line = true; // row is within [start.row, end.row]
                return (true, selected_line, at_head);
            }
        }
        (false, false, false)
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
                let next = cursor.move_up_once(anchor, self.point.column, buffer);
                self.update(buffer, &next);
            }
        }
    }

    pub fn move_down(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            let cursors = self.selections.clone();
            for cursor in cursors.iter() {
                let next = cursor.move_down_once(anchor, self.point.column, buffer);
                self.update(buffer, &next);
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

    pub fn move_to_start_of_previous_line(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let next = cursor
                .clone()
                .move_to_start_of_previous_line(anchor, buffer);
            self.update(buffer, &next);
        }
    }

    pub fn move_to_end_of_previous_line(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let next = cursor.clone().move_to_end_of_previous_line(anchor, buffer);
            self.update(buffer, &next);
        }
    }

    pub fn move_to_start_of_next_line(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let next = cursor.clone().move_to_start_of_next_line(anchor, buffer);
            self.update(buffer, &next);
        }
    }

    pub fn move_to_end_of_next_line(&mut self, anchor: bool, buffer: &Buffer) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let next = cursor.clone().move_to_end_of_next_line(anchor, buffer);
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
        till: bool,
        buffer: &Buffer,
    ) {
        let cursors = self.selections.clone();
        for cursor in cursors.iter() {
            let next = cursor
                .clone()
                .find_character(anchor, count, ch, forward, till, buffer);
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

    pub fn move_to_word(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            let cursors = self.selections.clone();
            for cursor in cursors.iter() {
                let next = cursor.clone().move_to_word(anchor, buffer);
                self.update(buffer, &next);
            }
        }
    }

    pub fn move_to_word_end(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            let cursors = self.selections.clone();
            for cursor in cursors.iter() {
                let next = cursor.clone().move_to_word_end(anchor, buffer);
                self.update(buffer, &next);
            }
        }
    }

    pub fn move_to_previous_word(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            let cursors = self.selections.clone();
            for cursor in cursors.iter() {
                let next = cursor.clone().move_to_previous_word(anchor, buffer);
                self.update(buffer, &next);
            }
        }
    }

    pub fn move_to_next_word(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            let cursors = self.selections.clone();
            for cursor in cursors.iter() {
                let next = cursor.clone().move_to_next_word(anchor, buffer);
                self.update(buffer, &next);
            }
        }
    }

    pub fn move_to_next_word_end(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            let cursors = self.selections.clone();
            for cursor in cursors.iter() {
                let next = cursor.clone().move_to_next_word_end(anchor, buffer);
                self.update(buffer, &next);
            }
        }
    }

    pub fn move_to_previous_word_end(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            let cursors = self.selections.clone();
            for cursor in cursors.iter() {
                let next = cursor.clone().move_to_previous_word_end(anchor, buffer);
                self.update(buffer, &next);
            }
        }
    }

    pub fn move_to_big_word(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            let cursors = self.selections.clone();
            for cursor in cursors.iter() {
                let next = cursor.clone().move_to_big_word(anchor, buffer);
                self.update(buffer, &next);
            }
        }
    }

    pub fn move_to_previous_big_word(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            let cursors = self.selections.clone();
            for cursor in cursors.iter() {
                let next = cursor.clone().move_to_previous_big_word(anchor, buffer);
                self.update(buffer, &next);
            }
        }
    }

    pub fn move_to_big_word_end(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            let cursors = self.selections.clone();
            for cursor in cursors.iter() {
                let next = cursor.clone().move_to_big_word_end(anchor, buffer);
                self.update(buffer, &next);
            }
        }
    }

    pub fn move_to_previous_big_word_end(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            let cursors = self.selections.clone();
            for cursor in cursors.iter() {
                let next = cursor.clone().move_to_previous_big_word_end(anchor, buffer);
                self.update(buffer, &next);
            }
        }
    }

    pub fn move_to_previous_paragraph(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            let cursors = self.selections.clone();
            for cursor in cursors.iter() {
                let next = cursor.clone().move_to_previous_paragraph(anchor, buffer);
                self.update(buffer, &next);
            }
        }
    }

    pub fn move_to_next_paragraph(&mut self, anchor: bool, count: u32, buffer: &Buffer) {
        for _ in 0..count {
            let cursors = self.selections.clone();
            for cursor in cursors.iter() {
                let next = cursor.clone().move_to_next_paragraph(anchor, buffer);
                self.update(buffer, &next);
            }
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

    pub fn move_to_syntax_target<F>(
        &mut self,
        anchor: bool,
        count: u32,
        syntax_tree: &crate::treesitter::SyntaxTree,
        buffer: &Buffer,
        target: F,
    ) where
        F: Fn(&crate::treesitter::SyntaxTree, usize) -> Option<crate::treesitter::SyntaxNode>,
    {
        for _ in 0..count {
            let cursors = self.selections.clone();
            let mut moved = false;
            for cursor in cursors.iter() {
                if let Some(next) =
                    cursor.move_to_syntax_target(anchor, syntax_tree, buffer, &target)
                {
                    self.point = next.head().to_point(buffer);
                    self.update(buffer, &next);
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
    }

    pub fn move_to_syntax_target_end<F>(
        &mut self,
        anchor: bool,
        count: u32,
        syntax_tree: &crate::treesitter::SyntaxTree,
        buffer: &Buffer,
        target: F,
    ) where
        F: Fn(&crate::treesitter::SyntaxTree, usize) -> Option<crate::treesitter::SyntaxNode>,
    {
        for _ in 0..count {
            let cursors = self.selections.clone();
            let mut moved = false;
            for cursor in cursors.iter() {
                if let Some(next) =
                    cursor.move_to_syntax_target_end(anchor, syntax_tree, buffer, &target)
                {
                    self.point = next.head().to_point(buffer);
                    self.update(buffer, &next);
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clock::ReplicaId;
    use text::BufferId;

    fn selection(
        buffer: &Buffer,
        id: usize,
        start: usize,
        end: usize,
        reversed: bool,
    ) -> Selection<Anchor> {
        Selection {
            id,
            start: buffer.anchor_at(start, Bias::Left),
            end: buffer.anchor_at(end, Bias::Left),
            reversed,
            goal: SelectionGoal::None,
        }
    }

    #[test]
    fn selection_text_normalizes_direction_and_uses_inclusive_endpoints() {
        let buffer = Buffer::new(ReplicaId::LOCAL, BufferId::new(1).unwrap(), "abcdef");

        assert_eq!(selection(&buffer, 0, 1, 2, false).text(&buffer), "bc");
        assert_eq!(selection(&buffer, 0, 1, 2, true).text(&buffer), "bc");
        assert_eq!(selection(&buffer, 0, 2, 2, false).text(&buffer), "");
    }

    #[test]
    fn similar_cursor_check_ignores_range_direction() {
        let buffer = Buffer::new(ReplicaId::LOCAL, BufferId::new(1).unwrap(), "abcdef");
        let mut selections = SelectionCollection::new();
        selections.selections = vec![selection(&buffer, 0, 1, 3, false)];

        assert!(selections.has_similar_cursor(&selection(&buffer, 1, 1, 3, false), &buffer));
        assert!(selections.has_similar_cursor(&selection(&buffer, 1, 3, 1, false), &buffer));
        assert!(!selections.has_similar_cursor(&selection(&buffer, 1, 2, 4, false), &buffer));
    }

    #[test]
    fn collection_text_joins_non_empty_selections() {
        let buffer = Buffer::new(ReplicaId::LOCAL, BufferId::new(1).unwrap(), "abcdef");
        let mut selections = SelectionCollection::new();
        selections.selections = vec![
            selection(&buffer, 0, 0, 1, false),
            selection(&buffer, 1, 3, 4, false),
        ];

        assert_eq!(selections.text(&buffer), "ab\nde");
    }

    #[test]
    fn next_match_within_searches_only_the_requested_following_rows() {
        let buffer = Buffer::new(
            ReplicaId::LOCAL,
            BufferId::new(1).unwrap(),
            "zero\none\ntarget",
        );
        let mut cursor = selection(&buffer, 0, 0, 0, false);

        assert!(cursor
            .move_to_next_match_within("target", &buffer, 1)
            .is_none());

        let matched = cursor
            .move_to_next_match_within("target", &buffer, 2)
            .expect("match should be found two rows below");
        assert_eq!(matched.head().to_point(&buffer), Point::new(2, 0));
    }
}
