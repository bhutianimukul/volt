//! Terminal cell grid — primary and alternate screen buffers.
//!
//! The grid stores rows of cells representing the visible terminal area.
//! Rows that scroll off the top are pushed into the scrollback buffer
//! (handled externally by the Terminal).
//!
//! Architecture note: In the final threading model, the grid is double-buffered
//! (parser writes "back" grid, renderer reads "front" grid, atomic swap at vsync).
//! This module implements a single grid; double-buffering is managed at a higher level.

use crate::cell::{Cell, CellFlags};
use crate::color::Color;

/// A single row of cells in the terminal grid.
#[derive(Debug, Clone)]
pub struct Row {
    cells: Vec<Cell>,
    /// Whether this row has been modified since last render.
    dirty: bool,
}

impl Row {
    /// Create a new row filled with default (blank) cells.
    pub fn new(cols: usize) -> Self {
        Self {
            cells: vec![Cell::default(); cols],
            dirty: true,
        }
    }

    /// Number of columns.
    #[inline]
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Whether the row has zero columns (shouldn't happen in practice).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Access a cell by column index.
    #[inline]
    pub fn get(&self, col: usize) -> Option<&Cell> {
        self.cells.get(col)
    }

    /// Mutably access a cell by column index. Marks row as dirty.
    #[inline]
    pub fn get_mut(&mut self, col: usize) -> Option<&mut Cell> {
        self.dirty = true;
        self.cells.get_mut(col)
    }

    /// Direct cell access by index.
    #[inline]
    pub fn cell(&self, col: usize) -> &Cell {
        &self.cells[col]
    }

    /// Direct mutable cell access. Marks row dirty.
    #[inline]
    pub fn cell_mut(&mut self, col: usize) -> &mut Cell {
        self.dirty = true;
        &mut self.cells[col]
    }

    /// Whether this row has been modified since last render.
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark this row as clean (rendered).
    #[inline]
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Mark this row as dirty (needs re-render).
    #[inline]
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Clear all cells to default. Marks row dirty.
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.reset();
        }
        self.dirty = true;
    }

    /// Clear all cells with a specific background color. Marks row dirty.
    pub fn clear_with_bg(&mut self, bg: Color) {
        for cell in &mut self.cells {
            cell.reset_with_bg(bg);
        }
        self.dirty = true;
    }

    /// Clear cells from `start` to end of row. Marks row dirty.
    pub fn clear_from(&mut self, start: usize, bg: Color) {
        for cell in self.cells.iter_mut().skip(start) {
            cell.reset_with_bg(bg);
        }
        self.dirty = true;
    }

    /// Clear cells from start of row to `end` (exclusive). Marks row dirty.
    pub fn clear_to(&mut self, end: usize, bg: Color) {
        for cell in self.cells.iter_mut().take(end) {
            cell.reset_with_bg(bg);
        }
        self.dirty = true;
    }

    /// Resize row to new column count, filling new cells with defaults.
    pub fn resize(&mut self, cols: usize) {
        self.cells.resize_with(cols, Cell::default);
        self.dirty = true;
    }

    /// Whether the row ends with a wrapline flag (line continues on next row).
    pub fn is_wrapped(&self) -> bool {
        self.cells
            .last()
            .is_some_and(|c| c.flags.contains(CellFlags::WRAPLINE))
    }

    /// Set the wrapline flag on the last cell.
    pub fn set_wrapped(&mut self, wrapped: bool) {
        if let Some(last) = self.cells.last_mut() {
            if wrapped {
                last.flags = last.flags.insert(CellFlags::WRAPLINE);
            } else {
                last.flags = last.flags.remove(CellFlags::WRAPLINE);
            }
            self.dirty = true;
        }
    }

    /// Get iterator over all cells.
    pub fn iter(&self) -> impl Iterator<Item = &Cell> {
        self.cells.iter()
    }

    /// Find the last non-empty column (for trimming trailing whitespace).
    pub fn last_occupied_col(&self) -> Option<usize> {
        self.cells.iter().rposition(|c| !c.is_empty())
    }
}

impl std::ops::Index<usize> for Row {
    type Output = Cell;

    #[inline]
    fn index(&self, col: usize) -> &Cell {
        &self.cells[col]
    }
}

impl std::ops::IndexMut<usize> for Row {
    #[inline]
    fn index_mut(&mut self, col: usize) -> &mut Cell {
        self.dirty = true;
        &mut self.cells[col]
    }
}

/// The terminal cell grid.
///
/// Stores the visible rows of the terminal. Does not include scrollback
/// (that's managed by `Scrollback`). Supports scroll regions, erase operations,
/// and resize with reflow.
#[derive(Debug, Clone)]
pub struct Grid {
    /// Rows of cells. Index 0 = top of screen.
    rows: Vec<Row>,
    /// Number of columns.
    cols: usize,
    /// Number of visible rows.
    num_rows: usize,
}

impl Grid {
    /// Create a new grid with the given dimensions, filled with blank cells.
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows: (0..rows).map(|_| Row::new(cols)).collect(),
            cols,
            num_rows: rows,
        }
    }

    /// Number of visible rows.
    #[inline]
    pub fn num_rows(&self) -> usize {
        self.num_rows
    }

    /// Number of columns.
    #[inline]
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Access a row by index.
    #[inline]
    pub fn row(&self, row: usize) -> &Row {
        &self.rows[row]
    }

    /// Mutably access a row by index.
    #[inline]
    pub fn row_mut(&mut self, row: usize) -> &mut Row {
        &mut self.rows[row]
    }

    /// Access a specific cell.
    #[inline]
    pub fn cell(&self, row: usize, col: usize) -> &Cell {
        &self.rows[row][col]
    }

    /// Mutably access a specific cell.
    #[inline]
    pub fn cell_mut(&mut self, row: usize, col: usize) -> &mut Cell {
        &mut self.rows[row][col]
    }

    /// Scroll the given region up by `count` lines.
    ///
    /// Lines at the top of the region are removed (returned for scrollback)
    /// and blank lines are inserted at the bottom.
    pub fn scroll_up(&mut self, region_top: usize, region_bottom: usize, count: usize, bg: Color) -> Vec<Row> {
        let count = count.min(region_bottom - region_top + 1);
        // Drain the lines that scroll off
        let scrolled_off: Vec<Row> = (0..count)
            .map(|_| {
                let row = self.rows.remove(region_top);
                row
            })
            .collect();

        // Insert blank lines at the bottom of the region
        let insert_at = region_bottom + 1 - count;
        for _ in 0..count {
            let mut new_row = Row::new(self.cols);
            if bg != Color::Default {
                new_row.clear_with_bg(bg);
            }
            self.rows.insert(insert_at, new_row);
        }

        // Mark affected rows dirty
        for row in region_top..=region_bottom {
            self.rows[row].mark_dirty();
        }

        scrolled_off
    }

    /// Scroll the given region down by `count` lines.
    ///
    /// Lines at the bottom of the region are discarded and blank lines
    /// are inserted at the top.
    pub fn scroll_down(&mut self, region_top: usize, region_bottom: usize, count: usize, bg: Color) {
        let count = count.min(region_bottom - region_top + 1);

        // Remove lines from bottom of region
        for _ in 0..count {
            self.rows.remove(region_bottom + 1 - count);
        }

        // Insert blank lines at top of region
        for _ in 0..count {
            let mut new_row = Row::new(self.cols);
            if bg != Color::Default {
                new_row.clear_with_bg(bg);
            }
            self.rows.insert(region_top, new_row);
        }

        // Mark affected rows dirty
        for row in region_top..=region_bottom {
            self.rows[row].mark_dirty();
        }
    }

    /// Clear the entire grid.
    pub fn clear(&mut self, bg: Color) {
        for row in &mut self.rows {
            if bg == Color::Default {
                row.clear();
            } else {
                row.clear_with_bg(bg);
            }
        }
    }

    /// Resize the grid. Simple version — truncates or extends rows/columns.
    /// Full reflow (re-wrapping long lines) is deferred to a later implementation.
    pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
        // Resize existing rows to new column count
        for row in &mut self.rows {
            row.resize(new_cols);
        }

        // Add or remove rows
        if new_rows > self.num_rows {
            for _ in 0..new_rows - self.num_rows {
                self.rows.push(Row::new(new_cols));
            }
        } else if new_rows < self.num_rows {
            self.rows.truncate(new_rows);
        }

        self.num_rows = new_rows;
        self.cols = new_cols;
    }

    /// Mark all rows as dirty (full redraw needed).
    pub fn mark_all_dirty(&mut self) {
        for row in &mut self.rows {
            row.mark_dirty();
        }
    }

    /// Mark all rows as clean.
    pub fn mark_all_clean(&mut self) {
        for row in &mut self.rows {
            row.mark_clean();
        }
    }

    /// Count how many rows are dirty.
    pub fn dirty_count(&self) -> usize {
        self.rows.iter().filter(|r| r.is_dirty()).count()
    }

    /// Iterate over all rows.
    pub fn iter_rows(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter()
    }
}

impl std::ops::Index<usize> for Grid {
    type Output = Row;

    #[inline]
    fn index(&self, row: usize) -> &Row {
        &self.rows[row]
    }
}

impl std::ops::IndexMut<usize> for Grid {
    #[inline]
    fn index_mut(&mut self, row: usize) -> &mut Row {
        &mut self.rows[row]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_dimensions() {
        let grid = Grid::new(24, 80);
        assert_eq!(grid.num_rows(), 24);
        assert_eq!(grid.cols(), 80);
    }

    #[test]
    fn grid_default_cells_empty() {
        let grid = Grid::new(24, 80);
        assert!(grid.cell(0, 0).is_empty());
    }

    #[test]
    fn grid_scroll_up() {
        let mut grid = Grid::new(5, 10);
        // Write something to first row
        grid.cell_mut(0, 0).c = 'A';
        grid.cell_mut(1, 0).c = 'B';

        let scrolled = grid.scroll_up(0, 4, 1, Color::Default);
        assert_eq!(scrolled.len(), 1);
        assert_eq!(scrolled[0][0].c, 'A');
        // Row with 'B' should now be at row 0
        assert_eq!(grid.cell(0, 0).c, 'B');
        // Last row should be blank
        assert!(grid.cell(4, 0).is_empty());
    }

    #[test]
    fn grid_scroll_down() {
        let mut grid = Grid::new(5, 10);
        grid.cell_mut(0, 0).c = 'A';
        grid.cell_mut(4, 0).c = 'E';

        grid.scroll_down(0, 4, 1, Color::Default);
        // Row 0 should now be blank
        assert!(grid.cell(0, 0).is_empty());
        // 'A' should be at row 1
        assert_eq!(grid.cell(1, 0).c, 'A');
        // 'E' was at row 4, which got removed
    }

    #[test]
    fn grid_scroll_with_region() {
        let mut grid = Grid::new(5, 10);
        grid.cell_mut(0, 0).c = 'A'; // Outside region
        grid.cell_mut(1, 0).c = 'B'; // Top of region
        grid.cell_mut(3, 0).c = 'D'; // Bottom of region
        grid.cell_mut(4, 0).c = 'E'; // Outside region

        let scrolled = grid.scroll_up(1, 3, 1, Color::Default);
        assert_eq!(scrolled[0][0].c, 'B');
        // Row outside region (0 and 4) unchanged
        assert_eq!(grid.cell(0, 0).c, 'A');
        assert_eq!(grid.cell(4, 0).c, 'E');
        // 'D' moved up within region
        assert_eq!(grid.cell(2, 0).c, 'D');
    }

    #[test]
    fn grid_resize() {
        let mut grid = Grid::new(24, 80);
        grid.cell_mut(0, 0).c = 'X';
        grid.resize(30, 100);
        assert_eq!(grid.num_rows(), 30);
        assert_eq!(grid.cols(), 100);
        assert_eq!(grid.cell(0, 0).c, 'X');
    }

    #[test]
    fn row_wrap_flag() {
        let mut row = Row::new(80);
        assert!(!row.is_wrapped());
        row.set_wrapped(true);
        assert!(row.is_wrapped());
        row.set_wrapped(false);
        assert!(!row.is_wrapped());
    }
}
