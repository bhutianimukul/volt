//! volt-renderer: Metal rendering pipeline, glyph atlas, and damage tracking.
//!
//! Uses `objc2-metal` (NOT deprecated `metal-rs`) for direct Metal API access.
//! Text pipeline uses `cosmic-text` (fontdb + harfrust shaping + swash rasterization).
//!
//! Render passes (instanced, one draw call per pass):
//! 1. Background colors
//! 2. Underlines/strikethrough/decorations
//! 3. Text glyphs (sampling atlas texture)
//! 4. Cursor + selection overlay

pub mod atlas;
pub mod pipeline;
pub mod shaders;
pub mod text;
pub mod damage;
pub mod renderer;
