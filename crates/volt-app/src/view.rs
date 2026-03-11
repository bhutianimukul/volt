//! NSView subclass hosting the CAMetalLayer for terminal rendering.
//!
//! Handles: mouse events, keyboard input dispatch, IME (NSTextInputClient for CJK),
//! drag-and-drop (Finder file → escaped path), and trackpad gestures.
