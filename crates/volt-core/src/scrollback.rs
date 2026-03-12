//! Scrollback buffer — VecDeque ring buffer of historical rows.
//!
//! Rows that scroll off the top of the visible grid are pushed here.
//! Uses VecDeque for O(1) push to back and pop from front (when max is reached).
//! Configurable max size (default 10,000 lines).

use std::collections::VecDeque;

use crate::grid::Row;

/// Default scrollback size in lines.
pub const DEFAULT_SCROLLBACK_LINES: usize = 10_000;

/// Ring buffer of scrollback rows.
#[derive(Debug, Clone)]
pub struct Scrollback {
    /// Historical rows. Front = oldest, back = most recent.
    rows: VecDeque<Row>,
    /// Maximum number of rows to retain.
    max_lines: usize,
}

impl Scrollback {
    /// Create a new scrollback buffer with the given maximum size.
    pub fn new(max_lines: usize) -> Self {
        Self {
            rows: VecDeque::new(),
            max_lines,
        }
    }

    /// Create with default max size.
    pub fn with_default_size() -> Self {
        Self::new(DEFAULT_SCROLLBACK_LINES)
    }

    /// Push a row into the scrollback. If at capacity, the oldest row is dropped.
    pub fn push(&mut self, row: Row) {
        if self.max_lines == 0 {
            return;
        }
        if self.rows.len() >= self.max_lines {
            self.rows.pop_front();
        }
        self.rows.push_back(row);
    }

    /// Push multiple rows (from a multi-line scroll).
    pub fn push_many(&mut self, rows: impl IntoIterator<Item = Row>) {
        for row in rows {
            self.push(row);
        }
    }

    /// Total number of scrollback rows.
    #[inline]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the scrollback is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Maximum scrollback size.
    #[inline]
    pub fn max_lines(&self) -> usize {
        self.max_lines
    }

    /// Access a scrollback row by index (0 = most recent, len-1 = oldest).
    /// Returns None if out of range.
    pub fn get(&self, offset: usize) -> Option<&Row> {
        if offset >= self.rows.len() {
            return None;
        }
        // offset 0 = most recent = back of deque
        let idx = self.rows.len() - 1 - offset;
        self.rows.get(idx)
    }

    /// Access a scrollback row by absolute index (0 = oldest).
    pub fn get_absolute(&self, index: usize) -> Option<&Row> {
        self.rows.get(index)
    }

    /// Clear all scrollback.
    pub fn clear(&mut self) {
        self.rows.clear();
    }

    /// Update the maximum scrollback size. Trims oldest rows if needed.
    pub fn set_max_lines(&mut self, max_lines: usize) {
        self.max_lines = max_lines;
        while self.rows.len() > max_lines {
            self.rows.pop_front();
        }
    }

    /// Iterate over all rows from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter()
    }

    /// Iterate from newest to oldest.
    pub fn iter_recent(&self) -> impl DoubleEndedIterator<Item = &Row> {
        self.rows.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_access() {
        let mut sb = Scrollback::new(100);
        sb.push(Row::new(80));
        sb.push(Row::new(80));
        assert_eq!(sb.len(), 2);
        assert!(sb.get(0).is_some()); // Most recent
        assert!(sb.get(1).is_some()); // Oldest
        assert!(sb.get(2).is_none()); // Out of range
    }

    #[test]
    fn max_lines_eviction() {
        let mut sb = Scrollback::new(3);
        for i in 0..5 {
            let mut row = Row::new(10);
            row.cell_mut(0).c = char::from(b'A' + i as u8);
            sb.push(row);
        }
        assert_eq!(sb.len(), 3);
        // Most recent should be 'E' (i=4)
        assert_eq!(sb.get(0).unwrap().cell(0).c, 'E');
        // Oldest remaining should be 'C' (i=2)
        assert_eq!(sb.get(2).unwrap().cell(0).c, 'C');
    }

    #[test]
    fn zero_scrollback() {
        let mut sb = Scrollback::new(0);
        sb.push(Row::new(80));
        assert_eq!(sb.len(), 0);
    }

    #[test]
    fn resize_max_lines() {
        let mut sb = Scrollback::new(100);
        for _ in 0..50 {
            sb.push(Row::new(80));
        }
        assert_eq!(sb.len(), 50);
        sb.set_max_lines(20);
        assert_eq!(sb.len(), 20);
    }
}
