//! volt-core: Terminal grid, VT parser, scrollback, selection, and terminal state.
//!
//! This is the core library for Volt. It wraps the `vte` crate for VT parsing
//! and manages all terminal state: the cell grid, cursor, modes, scrollback
//! buffer, selection, and damage tracking.

pub mod grid;
pub mod cell;
pub mod cursor;
pub mod parser;
pub mod scrollback;
pub mod selection;
pub mod modes;
pub mod damage;
pub mod color;
pub mod osc;

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
    pub size: TermSize,
    // TODO: grid, cursor, scrollback, modes, damage tracker
}

impl Terminal {
    pub fn new(size: TermSize) -> Self {
        Self { size }
    }
}
