use std::sync::Arc;
use text::{BufferSnapshot, Point};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WrapPoint {
    pub row: u32,
    pub column: u32,
}

impl WrapPoint {
    pub fn new(row: u32, column: u32) -> Self {
        Self { row, column }
    }
}

pub struct WrapMap {
    wrap_width: Option<u32>,
    snapshot: WrapSnapshot,
}

#[derive(Clone)]
pub struct WrapSnapshot {
    pub(crate) buffer: BufferSnapshot,
    pub(crate) wrap_width: Option<u32>,
    pub(crate) row_mappings: Arc<Vec<RowMapping>>,
}

#[derive(Debug, Clone)]
pub(crate) struct RowMapping {
    pub(crate) display_row_start: u32,
    pub(crate) wrap_indices: Vec<u32>, // columns in the buffer row where a new wrap line starts
}

impl WrapMap {
    pub fn new(buffer: BufferSnapshot, wrap_width: Option<u32>) -> Self {
        let mut map = Self {
            wrap_width,
            snapshot: WrapSnapshot {
                buffer: buffer.clone(),
                wrap_width,
                row_mappings: Arc::new(Vec::new()),
            },
        };
        map.sync(buffer);
        map
    }

    pub fn sync(&mut self, buffer: BufferSnapshot) {
        let mut row_mappings = Vec::with_capacity(buffer.row_count() as usize);
        let mut current_display_row = 0;

        for row in 0..buffer.row_count() {
            let line_len = buffer.line_len(row);
            let mut wrap_indices = Vec::new();
            wrap_indices.push(0);

            if let Some(width) = self.wrap_width {
                if width > 0 {
                    let mut current_col = width;
                    while current_col < line_len {
                        wrap_indices.push(current_col);
                        current_col += width;
                    }
                }
            }

            row_mappings.push(RowMapping {
                display_row_start: current_display_row,
                wrap_indices: wrap_indices.clone(),
            });
            current_display_row += wrap_indices.len() as u32;
        }

        self.snapshot = WrapSnapshot {
            buffer,
            wrap_width: self.wrap_width,
            row_mappings: Arc::new(row_mappings),
        };
    }

    pub fn snapshot(&self) -> WrapSnapshot {
        self.snapshot.clone()
    }

    pub fn set_wrap_width(&mut self, wrap_width: Option<u32>) {
        if self.wrap_width != wrap_width {
            self.wrap_width = wrap_width;
            self.sync(self.snapshot.buffer.clone());
        }
    }
}

impl WrapSnapshot {
    pub fn row_count(&self) -> u32 {
        if let Some(last) = self.row_mappings.last() {
            last.display_row_start + last.wrap_indices.len() as u32
        } else {
            0
        }
    }

    pub fn line_len(&self, display_row: u32) -> u32 {
        if self.row_mappings.is_empty() {
            return 0;
        }

        let buffer_row_idx = match self
            .row_mappings
            .binary_search_by_key(&display_row, |m| m.display_row_start)
        {
            Ok(idx) => idx,
            Err(idx) => (idx.saturating_sub(1)).min(self.row_mappings.len() - 1),
        };

        let mapping = &self.row_mappings[buffer_row_idx];
        let display_row_offset = display_row - mapping.display_row_start;
        let display_row_offset = (display_row_offset as usize).min(mapping.wrap_indices.len() - 1);

        if display_row_offset == mapping.wrap_indices.len() - 1 {
            // Last display row for this buffer row
            self.buffer.line_len(buffer_row_idx as u32) - mapping.wrap_indices[display_row_offset]
        } else {
            // Intermediate display row
            mapping.wrap_indices[display_row_offset + 1] - mapping.wrap_indices[display_row_offset]
        }
    }

    pub fn max_point(&self) -> WrapPoint {
        let last_mapping = self.row_mappings.last();
        if let Some(mapping) = last_mapping {
            let buffer_row = (self.row_mappings.len() - 1) as u32;
            let last_wrap_idx = *mapping.wrap_indices.last().unwrap();
            let line_len = self.buffer.line_len(buffer_row);
            WrapPoint::new(
                mapping.display_row_start + (mapping.wrap_indices.len() as u32 - 1),
                line_len - last_wrap_idx,
            )
        } else {
            WrapPoint::new(0, 0)
        }
    }

    pub fn to_wrap_point(&self, point: Point) -> WrapPoint {
        let mapping = match self.row_mappings.get(point.row as usize) {
            Some(m) => m,
            None => return WrapPoint::new(0, 0),
        };

        let mut display_row_offset = 0;
        let mut column_offset = point.column;

        for (i, &wrap_col) in mapping.wrap_indices.iter().enumerate().rev() {
            if point.column >= wrap_col {
                display_row_offset = i as u32;
                column_offset = point.column - wrap_col;
                break;
            }
        }

        WrapPoint::new(
            mapping.display_row_start + display_row_offset,
            column_offset,
        )
    }

    pub fn from_wrap_point(&self, point: WrapPoint) -> Point {
        if self.row_mappings.is_empty() {
            return Point::new(0, 0);
        }

        // Binary search for the buffer row that contains this display row
        let buffer_row_idx = match self
            .row_mappings
            .binary_search_by_key(&point.row, |m| m.display_row_start)
        {
            Ok(idx) => idx,
            Err(idx) => (idx.saturating_sub(1)).min(self.row_mappings.len() - 1),
        };

        let mapping = &self.row_mappings[buffer_row_idx];
        let display_row_offset = point.row - mapping.display_row_start;

        // Ensure display_row_offset is within bounds of wrap_indices
        let display_row_offset = (display_row_offset as usize).min(mapping.wrap_indices.len() - 1);
        let wrap_col = mapping.wrap_indices[display_row_offset];

        Point::new(buffer_row_idx as u32, wrap_col + point.column)
    }

    pub fn buffer_snapshot(&self) -> &BufferSnapshot {
        &self.buffer
    }

    pub fn wrap_width(&self) -> Option<u32> {
        self.wrap_width
    }
}
