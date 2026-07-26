use std::ops::Range;
use clock::ReplicaId;
use text::{Buffer, BufferId, BufferSnapshot, Point, ToPoint, Bias};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fold {
    pub start: Point,
    pub end: Point,
}

#[derive(Clone)]
pub struct PointMapping {
    pub original_range: Range<Point>,
    pub folded_range: Range<Point>,
    pub is_fold: bool,
}

#[derive(Clone)]
pub struct FoldMap {
    folds: Vec<Fold>,
    folded_buffer: BufferSnapshot,
    mappings: Vec<PointMapping>,
}

impl FoldMap {
    pub fn new(buffer: &BufferSnapshot, mut folds: Vec<Fold>) -> Self {
        folds.sort();
        // Remove nested or overlapping folds (keep outermost)
        let mut clean_folds: Vec<Fold> = Vec::new();
        for fold in folds {
            if let Some(last) = clean_folds.last() {
                if fold.start >= last.start && fold.start < last.end {
                    // Nested fold, skip for now
                    continue;
                }
            }
            if fold.start < fold.end {
                clean_folds.push(fold);
            }
        }

        let mut folded_text = String::new();
        let mut mappings = Vec::new();
        
        let mut current_orig = Point::zero();
        let mut current_fold = Point::zero();

        for fold in &clean_folds {
            // Text before fold
            if fold.start > current_orig {
                let chunk_range = current_orig..fold.start;
                let chunk: String = buffer.text_for_range(chunk_range.clone()).collect();
                folded_text.push_str(&chunk);

                let len_point = chunk_range.end - chunk_range.start;
                let next_fold = current_fold + len_point;
                mappings.push(PointMapping {
                    original_range: chunk_range,
                    folded_range: current_fold..next_fold,
                    is_fold: false,
                });
                current_orig = fold.start;
                current_fold = next_fold;
            }

            // Insert fold placeholder "{..}"
            let placeholder = "{..}";
            folded_text.push_str(placeholder);
            let next_fold = current_fold + Point::new(0, placeholder.len() as u32);
            mappings.push(PointMapping {
                original_range: fold.start..fold.end,
                folded_range: current_fold..next_fold,
                is_fold: true,
            });
            current_orig = fold.end;
            current_fold = next_fold;
        }

        // Remaining text
        let max_orig = buffer.max_point();
        if max_orig > current_orig {
            let chunk_range = current_orig..max_orig;
            let chunk: String = buffer.text_for_range(chunk_range.clone()).collect();
            folded_text.push_str(&chunk);

            let len_point = chunk_range.end - chunk_range.start;
            let next_fold = current_fold + len_point;
            mappings.push(PointMapping {
                original_range: chunk_range,
                folded_range: current_fold..next_fold,
                is_fold: false,
            });
        }

        let virtual_buffer = Buffer::new(ReplicaId::LOCAL, BufferId::new(9999).unwrap(), &folded_text);
        let folded_buffer = virtual_buffer.snapshot();

        Self {
            folds: clean_folds,
            folded_buffer: folded_buffer.clone(),
            mappings,
        }
    }

    pub fn folded_buffer(&self) -> &BufferSnapshot {
        &self.folded_buffer
    }

    pub fn to_folded_point(&self, point: Point) -> Point {
        for mapping in &self.mappings {
            if point >= mapping.original_range.start && point <= mapping.original_range.end {
                if mapping.is_fold {
                    return mapping.folded_range.start;
                } else {
                    let offset = point - mapping.original_range.start;
                    return mapping.folded_range.start + offset;
                }
            }
        }
        Point::zero()
    }

    pub fn from_folded_point(&self, point: Point) -> Point {
        for mapping in &self.mappings {
            if point >= mapping.folded_range.start && point <= mapping.folded_range.end {
                if mapping.is_fold {
                    return mapping.original_range.start;
                } else {
                    let offset = point - mapping.folded_range.start;
                    return mapping.original_range.start + offset;
                }
            }
        }
        Point::zero()
    }
}
