//! VT parser wrapping the `vte` crate.
//!
//! Implements `vte::Perform` to translate escape sequences into terminal state
//! mutations. The parser runs on a dedicated thread; parsed actions are applied
//! to the back grid buffer.
