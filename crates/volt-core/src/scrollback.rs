//! Scrollback buffer — compressed ring buffer of historical rows.
//!
//! Rows that scroll off the top of the grid are pushed into the scrollback.
//! Configurable max size. Rows may be compressed for memory efficiency.
