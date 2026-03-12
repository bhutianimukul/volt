//! Cursor state — position, style, visibility, and saved cursor state (DECSC/DECRC).

use crate::cell::CellTemplate;

/// Cursor position in the terminal grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPos {
    /// Row (0-indexed from top of visible area).
    pub row: usize,
    /// Column (0-indexed).
    pub col: usize,
}

impl Default for CursorPos {
    fn default() -> Self {
        Self { row: 0, col: 0 }
    }
}

/// Cursor visual style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    /// Filled block (█).
    Block,
    /// Underline (_).
    Underline,
    /// Vertical bar (|).
    Bar,
}

impl Default for CursorStyle {
    fn default() -> Self {
        Self::Block
    }
}

/// Terminal cursor state.
#[derive(Debug, Clone)]
pub struct Cursor {
    /// Current position.
    pub pos: CursorPos,
    /// Visual style (block, underline, bar).
    pub style: CursorStyle,
    /// Whether the cursor is visible (DECTCEM).
    pub visible: bool,
    /// Whether the cursor should blink.
    pub blinking: bool,
    /// Current SGR template — attributes applied to newly printed characters.
    pub template: CellTemplate,
    /// Origin mode (DECOM) — cursor movements relative to scroll region.
    pub origin_mode: bool,
    /// Pending wrap: cursor is at the right margin and next char should wrap.
    /// This is the "autowrap pending" state per DEC VT behavior.
    pub pending_wrap: bool,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            pos: CursorPos::default(),
            style: CursorStyle::default(),
            visible: true,
            blinking: true,
            template: CellTemplate::default(),
            origin_mode: false,
            pending_wrap: false,
        }
    }
}

impl Cursor {
    /// Move cursor to an absolute position, clamping to grid bounds.
    /// Clears pending wrap state.
    pub fn goto(&mut self, row: usize, col: usize, max_rows: usize, max_cols: usize) {
        self.pos.row = row.min(max_rows.saturating_sub(1));
        self.pos.col = col.min(max_cols.saturating_sub(1));
        self.pending_wrap = false;
    }

    /// Move cursor right by `n` columns, clamping to max.
    pub fn move_right(&mut self, n: usize, max_cols: usize) {
        self.pos.col = (self.pos.col + n).min(max_cols.saturating_sub(1));
        self.pending_wrap = false;
    }

    /// Move cursor left by `n` columns, clamping to 0.
    pub fn move_left(&mut self, n: usize) {
        self.pos.col = self.pos.col.saturating_sub(n);
        self.pending_wrap = false;
    }

    /// Move cursor up by `n` rows, clamping to top (or scroll region top).
    pub fn move_up(&mut self, n: usize, top: usize) {
        self.pos.row = self.pos.row.saturating_sub(n).max(top);
        self.pending_wrap = false;
    }

    /// Move cursor down by `n` rows, clamping to bottom (or scroll region bottom).
    pub fn move_down(&mut self, n: usize, bottom: usize) {
        self.pos.row = (self.pos.row + n).min(bottom);
        self.pending_wrap = false;
    }
}

/// Saved cursor state for DECSC/DECRC (ESC 7 / ESC 8).
///
/// Per the VT spec, DECSC saves: cursor position, character attributes (SGR),
/// character set designations, origin mode, and selective erase attribute.
#[derive(Debug, Clone)]
pub struct SavedCursor {
    pub pos: CursorPos,
    pub template: CellTemplate,
    pub origin_mode: bool,
    pub pending_wrap: bool,
}

impl Default for SavedCursor {
    fn default() -> Self {
        Self {
            pos: CursorPos::default(),
            template: CellTemplate::default(),
            origin_mode: false,
            pending_wrap: false,
        }
    }
}

impl SavedCursor {
    /// Save the current cursor state.
    pub fn save(cursor: &Cursor) -> Self {
        Self {
            pos: cursor.pos,
            template: cursor.template.clone(),
            origin_mode: cursor.origin_mode,
            pending_wrap: cursor.pending_wrap,
        }
    }

    /// Restore cursor state from this saved state.
    pub fn restore(&self, cursor: &mut Cursor) {
        cursor.pos = self.pos;
        cursor.template = self.template.clone();
        cursor.origin_mode = self.origin_mode;
        cursor.pending_wrap = self.pending_wrap;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_goto_clamps() {
        let mut cursor = Cursor::default();
        cursor.goto(100, 200, 24, 80);
        assert_eq!(cursor.pos.row, 23);
        assert_eq!(cursor.pos.col, 79);
    }

    #[test]
    fn cursor_move_clamps() {
        let mut cursor = Cursor::default();
        cursor.move_left(5); // Already at 0
        assert_eq!(cursor.pos.col, 0);

        cursor.move_right(100, 80);
        assert_eq!(cursor.pos.col, 79);
    }

    #[test]
    fn cursor_pending_wrap_cleared_on_move() {
        let mut cursor = Cursor::default();
        cursor.pending_wrap = true;
        cursor.move_right(1, 80);
        assert!(!cursor.pending_wrap);
    }

    #[test]
    fn save_restore_roundtrip() {
        let mut cursor = Cursor::default();
        cursor.pos = CursorPos { row: 5, col: 10 };
        cursor.template.fg = crate::color::Color::Named(crate::color::NamedColor::Red);

        let saved = SavedCursor::save(&cursor);
        let mut new_cursor = Cursor::default();
        saved.restore(&mut new_cursor);

        assert_eq!(new_cursor.pos, cursor.pos);
        assert_eq!(new_cursor.template.fg, cursor.template.fg);
    }
}
