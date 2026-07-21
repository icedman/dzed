use crate::display::wrap_map::{WrapMap, WrapPoint, WrapSnapshot};

use text::{BufferSnapshot, Point};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DisplayPoint(pub WrapPoint);

impl DisplayPoint {
    pub fn new(row: u32, column: u32) -> Self {
        Self(WrapPoint::new(row, column))
    }

    pub fn row(&self) -> u32 {
        self.0.row
    }

    pub fn column(&self) -> u32 {
        self.0.column
    }
}

pub struct DisplayMap {
    wrap_map: WrapMap,
    pub scroll_x: u32,
    pub scroll_y: u32,
    pub margin_left: u32,
    pub margin_right: u32,
    pub margin_top: u32,
    pub margin_bottom: u32,
}

pub struct DisplaySnapshot {
    pub(crate) wrap_snapshot: WrapSnapshot,
    pub scroll_x: u32,
    pub scroll_y: u32,
    pub margin_left: u32,
    pub margin_right: u32,
    pub margin_top: u32,
    pub margin_bottom: u32,
}

impl DisplayMap {
    pub fn new(buffer: BufferSnapshot, wrap_width: Option<u32>) -> Self {
        Self {
            wrap_map: WrapMap::new(buffer, wrap_width),
            scroll_x: 0,
            scroll_y: 0,
            margin_left: 0,
            margin_right: 0,
            margin_top: 0,
            margin_bottom: 0,
        }
    }

    pub fn snapshot(&self) -> DisplaySnapshot {
        DisplaySnapshot {
            wrap_snapshot: self.wrap_map.snapshot(),
            scroll_x: self.scroll_x,
            scroll_y: self.scroll_y,
            margin_left: self.margin_left,
            margin_right: self.margin_right,
            margin_top: self.margin_top,
            margin_bottom: self.margin_bottom,
        }
    }

    pub fn set_wrap_width(&mut self, width: Option<u32>) {
        self.wrap_map.set_wrap_width(width);
    }

    pub fn sync(&mut self, buffer: BufferSnapshot) {
        self.wrap_map.sync(buffer);
    }

    pub fn scroll_to_cursor(
        &mut self,
        display_cursor: DisplayPoint,
        screen_rows: i32,
        screen_cols: i32,
    ) {
        let cursor_row = display_cursor.row() as i32;
        let cursor_col = display_cursor.column() as i32;

        let visible_rows = (screen_rows - 1)
            .saturating_sub(self.margin_top as i32)
            .saturating_sub(self.margin_bottom as i32);
        let visible_cols = screen_cols
            .saturating_sub(self.margin_left as i32)
            .saturating_sub(self.margin_right as i32);

        // scroll based on cursor position
        let mut cursor_screen_row = cursor_row - self.scroll_y as i32;
        while cursor_screen_row >= visible_rows {
            self.scroll_y += 1;
            cursor_screen_row = cursor_row - self.scroll_y as i32;
        }
        while cursor_screen_row < 0 && self.scroll_y > 0 {
            self.scroll_y -= 1;
            cursor_screen_row = cursor_row - self.scroll_y as i32;
        }

        // Horizontal scroll only if not wrapping (or visible_cols is defined)
        if visible_cols > 0 {
            let mut cursor_screen_col = cursor_col - self.scroll_x as i32;
            while cursor_screen_col >= visible_cols {
                self.scroll_x += 1;
                cursor_screen_col = cursor_col - self.scroll_x as i32;
            }
            while cursor_screen_col < 0 && self.scroll_x > 0 {
                self.scroll_x -= 1;
                cursor_screen_col = cursor_col - self.scroll_x as i32;
            }
        }
    }
}

impl DisplaySnapshot {
    pub fn x(&self) -> u32 {
        return self.margin_left;
    }

    pub fn y(&self) -> u32 {
        self.margin_top
    }

    pub fn buffer_snapshot(&self) -> &BufferSnapshot {
        self.wrap_snapshot.buffer_snapshot()
    }

    pub fn row_count(&self) -> u32 {
        self.wrap_snapshot.row_count()
    }

    pub fn line_len(&self, row: u32) -> u32 {
        self.wrap_snapshot.line_len(row)
    }

    pub fn max_point(&self) -> DisplayPoint {
        DisplayPoint(self.wrap_snapshot.max_point())
    }

    pub fn point_to_display_point(&self, point: Point) -> DisplayPoint {
        DisplayPoint(self.wrap_snapshot.to_wrap_point(point))
    }

    pub fn display_point_to_point(&self, display_point: DisplayPoint) -> Point {
        self.wrap_snapshot.from_wrap_point(display_point.0)
    }

    /// Returns the buffer row for a given display row.
    pub fn buffer_row_for_display_row(&self, display_row: u32) -> u32 {
        self.display_point_to_point(DisplayPoint::new(display_row, 0))
            .row
    }

    /// Returns the range of buffer points covered by a display row.
    pub fn buffer_range_for_display_row(&self, display_row: u32) -> std::ops::Range<Point> {
        let start = self.display_point_to_point(DisplayPoint::new(display_row, 0));
        let end =
            self.display_point_to_point(DisplayPoint::new(display_row, self.line_len(display_row)));
        start..end
    }

    /// Returns the text for a given display row.
    pub fn line_text(&self, display_row: u32) -> String {
        let range = self.buffer_range_for_display_row(display_row);
        if range.is_empty() || range.start.row != range.end.row {
            return String::new();
        }

        self.buffer_snapshot()
            .text_for_range(range)
            .collect::<String>()
    }

    pub fn text_chunks(&self, display_row: u32) -> impl Iterator<Item = &str> {
        // For now, return a single chunk for the line.
        // In the future, this could return multiple chunks for syntax highlighting.
        std::iter::once(Box::leak(self.line_text(display_row).into_boxed_str()) as &str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clock::ReplicaId;
    use text::{Buffer, BufferId};

    #[test]
    fn extracts_wrapped_display_rows() {
        let buffer = Buffer::new(
            ReplicaId::LOCAL,
            BufferId::new(1).unwrap(),
            "abcdef\nsecond",
        );
        let display = DisplayMap::new(buffer.snapshot().clone(), Some(3)).snapshot();

        assert_eq!(display.row_count(), 4);
        assert_eq!(display.line_text(0), "abc");
        assert_eq!(display.line_text(1), "def");
        assert_eq!(display.line_text(2), "sec");
        assert_eq!(display.line_text(3), "ond");
    }

    #[test]
    fn extracts_utf8_text_by_buffer_points() {
        let buffer = Buffer::new(ReplicaId::LOCAL, BufferId::new(1).unwrap(), "aéøbc");
        let display = DisplayMap::new(buffer.snapshot().clone(), Some(3)).snapshot();

        assert_eq!(display.line_text(0), "aé");
        assert_eq!(display.line_text(1), "øb");
        assert_eq!(display.line_text(2), "c");
    }
}
