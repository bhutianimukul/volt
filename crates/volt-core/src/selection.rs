//! Selection model — normal, rectangular (Option+drag), word-wise, and line-wise.

/// Selection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    /// Normal character-by-character selection.
    Normal,
    /// Rectangular block selection (Option+drag on macOS).
    Block,
    /// Word-wise selection (double-click).
    Word,
    /// Line-wise selection (triple-click).
    Line,
}

/// A point in the terminal grid, including scrollback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionPoint {
    /// Row index. Positive = visible area (0 = top), negative = scrollback.
    /// We use i32 to represent both scrollback and visible area.
    pub row: i32,
    /// Column index.
    pub col: usize,
}

impl SelectionPoint {
    pub fn new(row: i32, col: usize) -> Self {
        Self { row, col }
    }

    /// Ordering: top-left is "less than" bottom-right.
    pub fn is_before(&self, other: &Self) -> bool {
        self.row < other.row || (self.row == other.row && self.col < other.col)
    }
}

/// Active selection state.
#[derive(Debug, Clone)]
pub struct Selection {
    /// Selection mode.
    pub mode: SelectionMode,
    /// Anchor point (where selection started).
    pub anchor: SelectionPoint,
    /// Current end point (where mouse/cursor currently is).
    pub end: SelectionPoint,
}

impl Selection {
    /// Create a new selection starting at the given point.
    pub fn new(mode: SelectionMode, point: SelectionPoint) -> Self {
        Self {
            mode,
            anchor: point,
            end: point,
        }
    }

    /// Update the end point of the selection.
    pub fn update(&mut self, point: SelectionPoint) {
        self.end = point;
    }

    /// Get the selection bounds normalized so start <= end.
    pub fn bounds(&self) -> (SelectionPoint, SelectionPoint) {
        if self.anchor.is_before(&self.end) {
            (self.anchor, self.end)
        } else {
            (self.end, self.anchor)
        }
    }

    /// Check if a given point is within the selection.
    pub fn contains(&self, row: i32, col: usize) -> bool {
        let (start, end) = self.bounds();

        match self.mode {
            SelectionMode::Normal => {
                if row < start.row || row > end.row {
                    return false;
                }
                if row == start.row && row == end.row {
                    col >= start.col && col <= end.col
                } else if row == start.row {
                    col >= start.col
                } else if row == end.row {
                    col <= end.col
                } else {
                    true
                }
            }
            SelectionMode::Block => {
                let (left, right) = if start.col <= end.col {
                    (start.col, end.col)
                } else {
                    (end.col, start.col)
                };
                row >= start.row && row <= end.row && col >= left && col <= right
            }
            SelectionMode::Word | SelectionMode::Line => {
                // Word and line selections expand the bounds at a higher level;
                // the raw contains check is the same as Normal.
                if row < start.row || row > end.row {
                    return false;
                }
                if row == start.row && row == end.row {
                    col >= start.col && col <= end.col
                } else if row == start.row {
                    col >= start.col
                } else if row == end.row {
                    col <= end.col
                } else {
                    true
                }
            }
        }
    }

    /// Whether the selection is empty (anchor == end).
    pub fn is_empty(&self) -> bool {
        self.anchor == self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_bounds_normalized() {
        let sel = Selection::new(
            SelectionMode::Normal,
            SelectionPoint::new(5, 10),
        );
        let mut sel = sel;
        sel.update(SelectionPoint::new(2, 5));
        let (start, end) = sel.bounds();
        assert_eq!(start.row, 2);
        assert_eq!(end.row, 5);
    }

    #[test]
    fn normal_selection_contains() {
        let mut sel = Selection::new(
            SelectionMode::Normal,
            SelectionPoint::new(1, 5),
        );
        sel.update(SelectionPoint::new(3, 10));

        // Middle row — all columns included
        assert!(sel.contains(2, 0));
        assert!(sel.contains(2, 79));

        // Start row — only from col 5
        assert!(!sel.contains(1, 4));
        assert!(sel.contains(1, 5));

        // End row — only to col 10
        assert!(sel.contains(3, 10));
        assert!(!sel.contains(3, 11));
    }

    #[test]
    fn block_selection_contains() {
        let mut sel = Selection::new(
            SelectionMode::Block,
            SelectionPoint::new(1, 5),
        );
        sel.update(SelectionPoint::new(3, 10));

        // Inside block
        assert!(sel.contains(2, 7));
        // Outside block column range
        assert!(!sel.contains(2, 3));
        assert!(!sel.contains(2, 11));
    }

    #[test]
    fn empty_selection() {
        let sel = Selection::new(
            SelectionMode::Normal,
            SelectionPoint::new(1, 5),
        );
        assert!(sel.is_empty());
    }
}
