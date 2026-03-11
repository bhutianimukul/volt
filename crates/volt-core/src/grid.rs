//! Terminal cell grid — primary and alternate screen buffers.
//!
//! The grid is double-buffered for the decoupled parser thread architecture:
//! the parser writes to the "back" grid while the renderer reads the "front" grid.
//! An atomic swap occurs at vsync.
