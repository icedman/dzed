use crate::document::{BufferText, Document, WordOffsets};
use rope::Point;
use std::{cmp::Ordering, io, ops::Range};
use sum_tree::Bias;
use text::{Anchor, Buffer, BufferId, Selection, SelectionGoal, ToOffset, ToPoint};

pub trait Movement {
    fn move_left(&mut self, anchor: bool);
    fn move_right(&mut self, anchor: bool);
    fn move_up(&mut self, anchor: bool);
    fn move_down(&mut self, anchor: bool);
    fn move_to_previous_word(&mut self, anchor: bool);
    fn move_to_next_word(&mut self, anchor: bool);
    fn move_to_start_of_line(&mut self, anchor: bool);
    fn move_to_end_of_line(&mut self, anchor: bool);
    fn move_to_start_of_document(&mut self, anchor: bool);
    fn move_to_end_of_document(&mut self, anchor: bool);
}
