use crate::document::{BufferText, Document, WordOffsets};
use rope::Point;
use std::{cmp::Ordering, io, ops::Range};
use sum_tree::Bias;
use text::{Anchor, Buffer, BufferId, Selection, SelectionGoal, ToOffset, ToPoint};

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    Visual_Line,
}
