//! Top-level renderer — orchestrates atlas, pipeline, text, and damage tracking
//! to produce frames via CAMetalLayer.
//!
//! Driven by CAMetalDisplayLink (macOS 14+) for ProMotion 120Hz support.
//! Adaptive frame coalescing: drains PTY, renders once per vsync.
