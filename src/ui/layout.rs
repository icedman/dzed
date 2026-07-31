#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeConstraint {
    Fixed(u16),
    Percentage(f32),
}

#[derive(Debug, Clone)]
pub enum LayoutNode {
    Leaf {
        window_id: usize,
    },
    Split {
        direction: SplitDirection,
        constraints: Vec<SizeConstraint>,
        children: Vec<LayoutNode>,
    },
}

impl LayoutNode {
    /// Recursively computes the rects for all leaf windows under this node given a parent rect.
    pub fn compute_layout(&self, rect: Rect) -> Vec<(usize, Rect)> {
        let mut results = Vec::new();
        self.compute_layout_recursive(rect, &mut results);
        results
    }

    fn compute_layout_recursive(&self, rect: Rect, results: &mut Vec<(usize, Rect)>) {
        match self {
            LayoutNode::Leaf { window_id } => {
                results.push((*window_id, rect));
            }
            LayoutNode::Split {
                direction,
                constraints,
                children,
            } => {
                if children.is_empty() {
                    return;
                }

                // If constraints don't match children count, assume equal percentage weights
                let actual_constraints = if constraints.len() == children.len() {
                    constraints.clone()
                } else {
                    vec![SizeConstraint::Percentage(1.0); children.len()]
                };

                let mut current_x = rect.x;
                let mut current_y = rect.y;
                let count = children.len();

                // Compute exact sizes
                let total_size = match direction {
                    SplitDirection::Horizontal => rect.width,
                    SplitDirection::Vertical => rect.height,
                };

                let mut fixed_sum = 0u16;
                let mut percent_weight_sum = 0.0f32;

                for c in &actual_constraints {
                    match c {
                        SizeConstraint::Fixed(val) => fixed_sum = fixed_sum.saturating_add(*val),
                        SizeConstraint::Percentage(weight) => percent_weight_sum += weight,
                    }
                }

                let remaining_size = total_size.saturating_sub(fixed_sum);
                let mut allocated_size = 0u16;

                for i in 0..count {
                    let constraint = actual_constraints[i];
                    let size = if i == count - 1 {
                        total_size.saturating_sub(allocated_size)
                    } else {
                        match constraint {
                            SizeConstraint::Fixed(val) => val,
                            SizeConstraint::Percentage(weight) => {
                                if percent_weight_sum > 0.0 {
                                    ((weight / percent_weight_sum) * remaining_size as f32).round()
                                        as u16
                                } else {
                                    0
                                }
                            }
                        }
                    };
                    allocated_size = allocated_size.saturating_add(size);

                    let child_rect = match direction {
                        SplitDirection::Horizontal => Rect {
                            x: current_x,
                            y: current_y,
                            width: size,
                            height: rect.height,
                        },
                        SplitDirection::Vertical => Rect {
                            x: current_x,
                            y: current_y,
                            width: rect.width,
                            height: size,
                        },
                    };

                    children[i].compute_layout_recursive(child_rect, results);

                    match direction {
                        SplitDirection::Horizontal => current_x = current_x.saturating_add(size),
                        SplitDirection::Vertical => current_y = current_y.saturating_add(size),
                    }
                }
            }
        }
    }

    pub fn split_leaf(&mut self, target_id: usize, new_id: usize, direction: SplitDirection) -> bool {
        match self {
            LayoutNode::Leaf { window_id } => {
                if *window_id == target_id {
                    *self = LayoutNode::Split {
                        direction,
                        constraints: vec![
                            SizeConstraint::Percentage(0.5),
                            SizeConstraint::Percentage(0.5),
                        ],
                        children: vec![
                            LayoutNode::Leaf { window_id: target_id },
                            LayoutNode::Leaf { window_id: new_id },
                        ],
                    };
                    true
                } else {
                    false
                }
            }
            LayoutNode::Split { children, .. } => {
                for child in children {
                    if child.split_leaf(target_id, new_id, direction) {
                        return true;
                    }
                }
                false
            }
        }
    }

    pub fn remove_leaf(&mut self, target_id: usize) -> (bool, Option<usize>) {
        match self {
            LayoutNode::Leaf { window_id } => {
                if *window_id == target_id {
                    (true, None)
                } else {
                    (false, None)
                }
            }
            LayoutNode::Split { constraints, children, .. } => {
                let mut remove_idx = None;
                for (i, child) in children.iter().enumerate() {
                    if let LayoutNode::Leaf { window_id } = child {
                        if *window_id == target_id {
                            remove_idx = Some(i);
                            break;
                        }
                    }
                }
                if let Some(idx) = remove_idx {
                    children.remove(idx);
                    if constraints.len() > idx {
                        constraints.remove(idx);
                    }
                    if children.len() == 1 {
                        let remaining_child = children.remove(0);
                        *self = remaining_child;
                        let sibling_id = match self {
                            LayoutNode::Leaf { window_id } => Some(*window_id),
                            _ => None,
                        };
                        return (true, sibling_id);
                    }
                    return (true, None);
                }
                for child in children.iter_mut() {
                    let (removed, sibling) = child.remove_leaf(target_id);
                    if removed {
                        return (true, sibling);
                    }
                }
                (false, None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_and_remove_nested_leaves() {
        // Start with window 1
        let mut root = LayoutNode::Leaf { window_id: 1 };

        // Split window 1 vertically to create 1 and 2
        assert!(root.split_leaf(1, 2, SplitDirection::Horizontal));

        // Split window 2 horizontally to create 2 and 3 (nested split)
        assert!(root.split_leaf(2, 3, SplitDirection::Vertical));

        // Layout tree should be:
        // Split(Horizontal)
        //   - Leaf { window_id: 1 }
        //   - Split(Vertical)
        //       - Leaf { window_id: 2 }
        //       - Leaf { window_id: 3 }

        // Remove leaf 3
        let (removed, sibling) = root.remove_leaf(3);
        assert!(removed);
        // Sibling of 3 was 2, so it collapses to Leaf{2}
        assert_eq!(sibling, Some(2));

        // Ensure Leaf 1 is still present in the tree
        if let LayoutNode::Split { children, .. } = &root {
            assert_eq!(children.len(), 2);
            assert!(matches!(children[0], LayoutNode::Leaf { window_id: 1 }));
            assert!(matches!(children[1], LayoutNode::Leaf { window_id: 2 }));
        } else {
            panic!("Expected split root");
        }
    }
}
