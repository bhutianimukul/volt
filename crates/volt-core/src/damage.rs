//! Damage tracking — scroll-aware dirty bit tracking for efficient rendering.
//!
//! The damage tracker maintains two kinds of state:
//! 1. **Content dirty bits**: per-row flags indicating cells changed since last render
//! 2. **Scroll delta**: accumulated scroll offset since last render
//!
//! The renderer uses scroll delta to shift existing instance buffer data via memcpy,
//! then only rebuilds rows with content-dirty bits. This makes scrolling O(new_rows)
//! instead of O(all_rows).
//!
//! A full redraw is triggered when >30% of rows are content-dirty (at that point
//! rebuilding everything is cheaper than selective updates).

use bitvec::prelude::*;

/// Tracks which parts of the terminal have changed since the last render.
#[derive(Debug, Clone)]
pub struct DamageTracker {
    /// Per-row dirty flags. true = row content changed.
    dirty_rows: BitVec,
    /// Accumulated scroll delta since last render.
    /// Positive = scrolled up (new content at bottom), negative = scrolled down.
    scroll_delta: i32,
    /// Total number of rows.
    num_rows: usize,
    /// Threshold (0.0-1.0) above which a full redraw is more efficient.
    full_redraw_threshold: f32,
}

impl DamageTracker {
    /// Create a new damage tracker. All rows start dirty (initial render).
    pub fn new(num_rows: usize) -> Self {
        let mut dirty_rows = BitVec::with_capacity(num_rows);
        dirty_rows.resize(num_rows, true);
        Self {
            dirty_rows,
            scroll_delta: 0,
            num_rows,
            full_redraw_threshold: 0.3,
        }
    }

    /// Mark a specific row as dirty.
    #[inline]
    pub fn mark_dirty(&mut self, row: usize) {
        if row < self.num_rows {
            self.dirty_rows.set(row, true);
        }
    }

    /// Mark a range of rows as dirty (inclusive).
    pub fn mark_range_dirty(&mut self, start: usize, end: usize) {
        let end = end.min(self.num_rows.saturating_sub(1));
        for row in start..=end {
            self.dirty_rows.set(row, true);
        }
    }

    /// Mark all rows as dirty (full redraw).
    pub fn mark_all_dirty(&mut self) {
        self.dirty_rows.fill(true);
    }

    /// Record a scroll event. `delta` positive = scrolled up, negative = scrolled down.
    ///
    /// Only marks the newly exposed rows as content-dirty (not all rows).
    pub fn record_scroll(&mut self, delta: i32) {
        self.scroll_delta += delta;

        if delta > 0 {
            // Scrolled up: new rows appeared at the bottom
            let new_start = self.num_rows.saturating_sub(delta as usize);
            self.mark_range_dirty(new_start, self.num_rows.saturating_sub(1));
        } else if delta < 0 {
            // Scrolled down: new rows appeared at the top
            let new_end = (-delta) as usize;
            self.mark_range_dirty(0, new_end.min(self.num_rows).saturating_sub(1));
        }
    }

    /// Query the render state: returns what the renderer needs to know.
    pub fn render_state(&self) -> DamageState {
        let dirty_count = self.dirty_rows.count_ones();
        let dirty_ratio = dirty_count as f32 / self.num_rows as f32;

        if dirty_count == 0 && self.scroll_delta == 0 {
            DamageState::Clean
        } else if dirty_ratio > self.full_redraw_threshold {
            DamageState::FullRedraw
        } else {
            DamageState::Partial {
                scroll_delta: self.scroll_delta,
                dirty_rows: self
                    .dirty_rows
                    .iter()
                    .enumerate()
                    .filter(|(_, b)| *b.as_ref())
                    .map(|(i, _)| i)
                    .collect(),
            }
        }
    }

    /// Is a specific row dirty?
    #[inline]
    pub fn is_dirty(&self, row: usize) -> bool {
        row < self.num_rows && self.dirty_rows[row]
    }

    /// Number of dirty rows.
    pub fn dirty_count(&self) -> usize {
        self.dirty_rows.count_ones()
    }

    /// Current scroll delta.
    pub fn scroll_delta(&self) -> i32 {
        self.scroll_delta
    }

    /// Reset all damage state after a render. Called by the renderer.
    pub fn reset(&mut self) {
        self.dirty_rows.fill(false);
        self.scroll_delta = 0;
    }

    /// Resize the tracker for a new row count. Marks all dirty.
    pub fn resize(&mut self, num_rows: usize) {
        self.num_rows = num_rows;
        self.dirty_rows.resize(num_rows, true);
        self.dirty_rows.fill(true);
        self.scroll_delta = 0;
    }
}

/// What the renderer should do based on current damage.
#[derive(Debug, Clone, PartialEq)]
pub enum DamageState {
    /// Nothing changed — skip render.
    Clean,
    /// Too many dirty rows — rebuild entire instance buffer.
    FullRedraw,
    /// Partial update: shift by scroll_delta, rebuild only listed rows.
    Partial {
        scroll_delta: i32,
        dirty_rows: Vec<usize>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tracker_all_dirty() {
        let tracker = DamageTracker::new(24);
        assert_eq!(tracker.dirty_count(), 24);
    }

    #[test]
    fn reset_clears_all() {
        let mut tracker = DamageTracker::new(24);
        tracker.reset();
        assert_eq!(tracker.dirty_count(), 0);
        assert_eq!(tracker.scroll_delta(), 0);
        assert_eq!(tracker.render_state(), DamageState::Clean);
    }

    #[test]
    fn scroll_marks_new_rows_dirty() {
        let mut tracker = DamageTracker::new(24);
        tracker.reset();
        // Scroll up by 2: rows 22-23 should be dirty
        tracker.record_scroll(2);
        assert!(tracker.is_dirty(22));
        assert!(tracker.is_dirty(23));
        assert!(!tracker.is_dirty(0));
        assert_eq!(tracker.scroll_delta(), 2);
    }

    #[test]
    fn partial_vs_full_redraw() {
        let mut tracker = DamageTracker::new(10);
        tracker.reset();
        // Mark 2 rows dirty (20%) → partial
        tracker.mark_dirty(0);
        tracker.mark_dirty(5);
        match tracker.render_state() {
            DamageState::Partial { dirty_rows, .. } => {
                assert_eq!(dirty_rows, vec![0, 5]);
            }
            _ => panic!("Expected Partial"),
        }

        // Mark 4 total (40% > 30% threshold) → full redraw
        tracker.mark_dirty(2);
        tracker.mark_dirty(7);
        assert_eq!(tracker.render_state(), DamageState::FullRedraw);
    }
}
