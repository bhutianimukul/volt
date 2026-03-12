//! volt-core: Terminal grid, VT parser, scrollback, selection, and terminal state.
//!
//! This is the core library for Volt. It wraps the `vte` crate for VT parsing
//! and manages all terminal state: the cell grid, cursor, modes, scrollback
//! buffer, selection, and damage tracking.
//!
//! `volt-core` is platform-agnostic — it has no macOS dependencies. The grid,
//! parser, and all terminal state logic live here.

pub mod cell;
pub mod color;
pub mod cursor;
pub mod damage;
pub mod grid;
pub mod modes;
pub mod osc;
pub mod parser;
pub mod scrollback;
pub mod selection;

use cursor::{Cursor, SavedCursor};
use damage::DamageTracker;
use grid::Grid;
use modes::{ScrollRegion, TerminalModes};
use parser::{Handler, TerminalEvent};
use scrollback::Scrollback;
use selection::Selection;

/// Terminal dimensions in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermSize {
    pub cols: u16,
    pub rows: u16,
}

/// The main terminal state machine.
///
/// Owns the grid, cursor, scrollback, modes, and damage tracker.
/// Fed bytes from the PTY via the VT parser.
pub struct Terminal {
    /// Visible grid (primary or alternate, depending on mode).
    grid: Grid,
    /// Alternate screen grid.
    alt_grid: Grid,
    /// Cursor state.
    cursor: Cursor,
    /// Terminal modes (DEC private, ANSI, mouse, etc.).
    modes: TerminalModes,
    /// Scroll region.
    scroll_region: ScrollRegion,
    /// Saved cursor for primary screen (DECSC/DECRC).
    saved_cursor: SavedCursor,
    /// Saved cursor for alternate screen.
    alt_saved_cursor: SavedCursor,
    /// Scrollback buffer.
    scrollback: Scrollback,
    /// Damage tracker for the renderer.
    damage: DamageTracker,
    /// Active selection (if any).
    selection: Option<Selection>,
    /// VTE parser state machine.
    vte_parser: vte::Parser,
    /// Terminal dimensions.
    size: TermSize,
    /// Pending events for the host application.
    pending_events: Vec<TerminalEvent>,
}

impl Terminal {
    /// Create a new terminal with the given dimensions.
    pub fn new(size: TermSize) -> Self {
        let rows = size.rows as usize;
        let cols = size.cols as usize;

        Self {
            grid: Grid::new(rows, cols),
            alt_grid: Grid::new(rows, cols),
            cursor: Cursor::default(),
            modes: TerminalModes::default(),
            scroll_region: ScrollRegion::full(rows),
            saved_cursor: SavedCursor::default(),
            alt_saved_cursor: SavedCursor::default(),
            scrollback: Scrollback::with_default_size(),
            damage: DamageTracker::new(rows),
            selection: None,
            vte_parser: vte::Parser::new(),
            size,
            pending_events: Vec::new(),
        }
    }

    /// Feed raw bytes from the PTY into the terminal.
    ///
    /// Bytes are parsed by the VTE state machine and applied to the terminal state.
    /// Returns any events that need host-level handling (title changes, clipboard, etc.).
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<TerminalEvent> {
        self.pending_events.clear();
        let mut scrollback_rows = Vec::new();

        let mut handler = Handler {
            grid: &mut self.grid,
            cursor: &mut self.cursor,
            modes: &mut self.modes,
            scroll_region: &mut self.scroll_region,
            saved_cursor: &mut self.saved_cursor,
            alt_saved_cursor: &mut self.alt_saved_cursor,
            alt_grid: &mut self.alt_grid,
            scrollback_rows: &mut scrollback_rows,
            events: &mut self.pending_events,
            cols: self.size.cols as usize,
            rows: self.size.rows as usize,
        };
        self.vte_parser.advance(&mut handler, bytes);

        // Push any scrolled-off rows to scrollback
        self.scrollback.push_many(scrollback_rows);

        std::mem::take(&mut self.pending_events)
    }

    /// Get the current grid (primary or alternate depending on mode).
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Get the alternate grid.
    pub fn alt_grid(&self) -> &Grid {
        &self.alt_grid
    }

    /// Get the cursor state.
    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    /// Get the terminal modes.
    pub fn modes(&self) -> &TerminalModes {
        &self.modes
    }

    /// Get the scrollback buffer.
    pub fn scrollback(&self) -> &Scrollback {
        &self.scrollback
    }

    /// Get the damage tracker.
    pub fn damage(&self) -> &DamageTracker {
        &self.damage
    }

    /// Get a mutable reference to the damage tracker.
    pub fn damage_mut(&mut self) -> &mut DamageTracker {
        &mut self.damage
    }

    /// Current terminal size.
    pub fn size(&self) -> TermSize {
        self.size
    }

    /// Get the scroll region.
    pub fn scroll_region(&self) -> &ScrollRegion {
        &self.scroll_region
    }

    /// Get the active selection, if any.
    pub fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    /// Set or clear the active selection.
    pub fn set_selection(&mut self, selection: Option<Selection>) {
        self.selection = selection;
    }

    /// Whether the terminal is in alternate screen mode.
    pub fn is_alternate_screen(&self) -> bool {
        self.modes.alternate_screen
    }

    /// Resize the terminal. Updates grids, scroll region, and clamps cursor.
    pub fn resize(&mut self, new_size: TermSize) {
        let rows = new_size.rows as usize;
        let cols = new_size.cols as usize;

        self.grid.resize(rows, cols);
        self.alt_grid.resize(rows, cols);
        self.scroll_region = ScrollRegion::full(rows);
        self.damage.resize(rows);
        self.size = new_size;

        // Clamp cursor
        self.cursor
            .goto(self.cursor.pos.row, self.cursor.pos.col, rows, cols);
    }

    /// Clear scrollback buffer.
    pub fn clear_scrollback(&mut self) {
        self.scrollback.clear();
    }

    /// Get a cell at the given position in the visible grid.
    pub fn cell(&self, row: usize, col: usize) -> &cell::Cell {
        self.grid.cell(row, col)
    }

    /// Extract the visible text content of a row as a String.
    pub fn row_text(&self, row: usize) -> String {
        let grid_row = self.grid.row(row);
        let mut text = String::with_capacity(self.size.cols as usize);
        for col in 0..grid_row.len() {
            let cell = &grid_row[col];
            if !cell.is_wide_spacer() {
                text.push(cell.c);
            }
        }
        // Trim trailing spaces
        text.truncate(text.trim_end().len());
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn term(rows: u16, cols: u16) -> Terminal {
        Terminal::new(TermSize { rows, cols })
    }

    #[test]
    fn new_terminal_is_blank() {
        let t = term(24, 80);
        assert_eq!(t.size().rows, 24);
        assert_eq!(t.size().cols, 80);
        assert!(t.cell(0, 0).is_empty());
        assert_eq!(t.cursor().pos.row, 0);
        assert_eq!(t.cursor().pos.col, 0);
    }

    #[test]
    fn print_text() {
        let mut t = term(24, 80);
        t.feed(b"Hello");
        assert_eq!(t.row_text(0), "Hello");
        assert_eq!(t.cursor().pos.col, 5);
    }

    #[test]
    fn cursor_movement() {
        let mut t = term(24, 80);
        t.feed(b"Hello");
        // CUP to row 3, col 10 (1-based)
        t.feed(b"\x1b[3;10H");
        assert_eq!(t.cursor().pos.row, 2);
        assert_eq!(t.cursor().pos.col, 9);
    }

    #[test]
    fn cursor_up_down_left_right() {
        let mut t = term(24, 80);
        t.feed(b"\x1b[10;10H"); // Go to row 10, col 10
        t.feed(b"\x1b[3A"); // Up 3
        assert_eq!(t.cursor().pos.row, 6); // 9 - 3
        t.feed(b"\x1b[2B"); // Down 2
        assert_eq!(t.cursor().pos.row, 8); // 6 + 2
        t.feed(b"\x1b[4D"); // Left 4
        assert_eq!(t.cursor().pos.col, 5); // 9 - 4
        t.feed(b"\x1b[10C"); // Right 10
        assert_eq!(t.cursor().pos.col, 15); // 5 + 10
    }

    #[test]
    fn sgr_bold_and_color() {
        let mut t = term(24, 80);
        // Set bold red foreground
        t.feed(b"\x1b[1;31mX");
        let cell = t.cell(0, 0);
        assert_eq!(cell.c, 'X');
        assert!(cell.flags.contains(cell::CellFlags::BOLD));
        assert_eq!(cell.fg, color::Color::Named(color::NamedColor::Red));
    }

    #[test]
    fn sgr_reset() {
        let mut t = term(24, 80);
        t.feed(b"\x1b[1;31mA\x1b[0mB");
        // 'A' should be bold+red
        assert!(t.cell(0, 0).flags.contains(cell::CellFlags::BOLD));
        // 'B' should be default
        assert!(t.cell(0, 1).flags.is_empty());
        assert_eq!(t.cell(0, 1).fg, color::Color::Default);
    }

    #[test]
    fn sgr_256_color() {
        let mut t = term(24, 80);
        t.feed(b"\x1b[38;5;196mR"); // Red from 256-color palette
        assert_eq!(t.cell(0, 0).fg, color::Color::Indexed(196));
    }

    #[test]
    fn sgr_rgb_color() {
        let mut t = term(24, 80);
        t.feed(b"\x1b[38;2;255;128;0mO"); // RGB orange
        assert_eq!(
            t.cell(0, 0).fg,
            color::Color::Rgb(color::Rgb::new(255, 128, 0))
        );
    }

    #[test]
    fn linefeed_and_carriage_return() {
        let mut t = term(24, 80);
        t.feed(b"Line1\r\nLine2");
        assert_eq!(t.row_text(0), "Line1");
        assert_eq!(t.row_text(1), "Line2");
    }

    #[test]
    fn erase_in_line() {
        let mut t = term(24, 80);
        t.feed(b"ABCDE");
        t.feed(b"\x1b[3D"); // Move left 3 (cursor now at col 2)
        t.feed(b"\x1b[0K"); // Erase from cursor to end of line
        assert_eq!(t.row_text(0), "AB");
    }

    #[test]
    fn erase_in_display() {
        let mut t = term(5, 10);
        t.feed(b"Line0\r\nLine1\r\nLine2");
        t.feed(b"\x1b[2J"); // Clear entire display
        for row in 0..5 {
            assert_eq!(t.row_text(row), "");
        }
    }

    #[test]
    fn scroll_up() {
        let mut t = term(3, 10);
        t.feed(b"AAA\r\nBBB\r\nCCC");
        // Cursor at bottom. Linefeed should scroll.
        t.feed(b"\r\nDDD");
        assert_eq!(t.row_text(0), "BBB");
        assert_eq!(t.row_text(1), "CCC");
        assert_eq!(t.row_text(2), "DDD");
        // 'AAA' should be in scrollback
        assert_eq!(t.scrollback().len(), 1);
    }

    #[test]
    fn scroll_region() {
        let mut t = term(5, 10);
        t.feed(b"Row0\r\nRow1\r\nRow2\r\nRow3\r\nRow4");
        // Set scroll region to rows 2-4 (1-based)
        t.feed(b"\x1b[2;4r");
        // Cursor moved to home by DECSTBM
        assert_eq!(t.cursor().pos.row, 0);
        assert_eq!(t.cursor().pos.col, 0);
    }

    #[test]
    fn alternate_screen() {
        let mut t = term(5, 10);
        t.feed(b"Primary");
        assert_eq!(t.row_text(0), "Primary");

        // Switch to alternate screen (mode 1049)
        t.feed(b"\x1b[?1049h");
        assert!(t.is_alternate_screen());

        // Move to home and write on alt screen
        t.feed(b"\x1b[H");
        t.feed(b"Alt");
        assert_eq!(t.row_text(0), "Alt");

        // Switch back
        t.feed(b"\x1b[?1049l");
        assert!(!t.is_alternate_screen());
        assert_eq!(t.row_text(0), "Primary");
    }

    #[test]
    fn backspace() {
        let mut t = term(24, 80);
        t.feed(b"AB\x08C"); // Print AB, backspace, print C (overwrites B)
        assert_eq!(t.row_text(0), "AC");
    }

    #[test]
    fn tab_stops() {
        let mut t = term(24, 80);
        t.feed(b"A\tB");
        assert_eq!(t.cursor().pos.col, 9); // Tab to 8, then 'B' at 9
        assert_eq!(t.cell(0, 0).c, 'A');
        assert_eq!(t.cell(0, 8).c, 'B');
    }

    #[test]
    fn autowrap() {
        let mut t = term(3, 5);
        t.feed(b"12345X"); // 5 chars fill row, 6th wraps
        assert_eq!(t.row_text(0), "12345");
        assert_eq!(t.row_text(1), "X");
    }

    #[test]
    fn save_restore_cursor() {
        let mut t = term(24, 80);
        t.feed(b"\x1b[5;10H"); // Move to row 5, col 10
        t.feed(b"\x1b7"); // Save cursor (DECSC)
        t.feed(b"\x1b[1;1H"); // Move to home
        t.feed(b"\x1b8"); // Restore cursor (DECRC)
        assert_eq!(t.cursor().pos.row, 4);
        assert_eq!(t.cursor().pos.col, 9);
    }

    #[test]
    fn osc_title() {
        let mut t = term(24, 80);
        let events = t.feed(b"\x1b]0;My Title\x07");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, TerminalEvent::TitleChanged(s) if s == "My Title"))
        );
    }

    #[test]
    fn osc_working_directory() {
        let mut t = term(24, 80);
        let events = t.feed(b"\x1b]7;file:///Users/test\x07");
        assert!(events.iter().any(
            |e| matches!(e, TerminalEvent::WorkingDirectoryChanged(s) if s == "file:///Users/test")
        ));
    }

    #[test]
    fn resize() {
        let mut t = term(24, 80);
        t.feed(b"Hello");
        t.resize(TermSize {
            rows: 30,
            cols: 100,
        });
        assert_eq!(t.size().rows, 30);
        assert_eq!(t.size().cols, 100);
        assert_eq!(t.row_text(0), "Hello");
    }

    #[test]
    fn insert_characters() {
        let mut t = term(24, 80);
        t.feed(b"ABCDE");
        t.feed(b"\x1b[3D"); // Move left 3 (cursor at col 2)
        t.feed(b"\x1b[2@"); // Insert 2 characters
        // "AB  CDE" (CD shifted right by 2, E may fall off if narrow)
        assert_eq!(t.cell(0, 0).c, 'A');
        assert_eq!(t.cell(0, 1).c, 'B');
        assert_eq!(t.cell(0, 2).c, ' ');
        assert_eq!(t.cell(0, 3).c, ' ');
        assert_eq!(t.cell(0, 4).c, 'C');
    }

    #[test]
    fn delete_characters() {
        let mut t = term(24, 80);
        t.feed(b"ABCDE");
        t.feed(b"\x1b[4D"); // Move left 4 (cursor at col 1)
        t.feed(b"\x1b[2P"); // Delete 2 characters
        // "ADEE" → "ADE" (BC deleted, DE shifted left)
        assert_eq!(t.cell(0, 0).c, 'A');
        assert_eq!(t.cell(0, 1).c, 'D');
        assert_eq!(t.cell(0, 2).c, 'E');
    }

    #[test]
    fn bell_event() {
        let mut t = term(24, 80);
        let events = t.feed(b"\x07");
        assert!(events.iter().any(|e| matches!(e, TerminalEvent::Bell)));
    }

    #[test]
    fn reverse_index() {
        let mut t = term(5, 10);
        // Cursor at row 0, reverse index should scroll down
        t.feed(b"\x1bM"); // RI
        // Row 0 should be blank (new row inserted at top)
        assert_eq!(t.row_text(0), "");
    }

    #[test]
    fn dec_screen_alignment() {
        let mut t = term(3, 5);
        t.feed(b"\x1b#8"); // DECALN — fill with 'E'
        for row in 0..3 {
            assert_eq!(t.row_text(row), "EEEEE");
        }
    }
}
