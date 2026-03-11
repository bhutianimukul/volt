//! Damage tracking — scroll-aware dirty bit tracking for efficient rendering.
//!
//! Uses a BitVec per row for content dirty bits plus a separate scroll offset
//! counter. On scroll, only new rows are marked content-dirty; existing rows
//! are shifted in the instance buffer via memcpy.
