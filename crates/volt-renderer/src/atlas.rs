//! Glyph atlas — 2048x2048 MTLTexture with LRU eviction.
//!
//! Pre-populates ASCII glyphs for all 4 font variants (regular/bold/italic/bold-italic)
//! on startup. HashMap from `(font_id, glyph_id, subpixel_bin)` → atlas region.
//! Grows to 4096x4096 if needed. Consider MTLTextureType2DArray with 4 layers.
//!
//! Reference: Alacritty's `atlas.rs` for packing strategy.
