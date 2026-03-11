//! Text pipeline — font discovery, shaping, and rasterization via cosmic-text.
//!
//! FontSystem is lazy-loaded on a background thread for <50ms cold start.
//! Handles: font fallback chains, emoji (Apple Color Emoji), Nerd Font/Powerline,
//! bold/italic variants, ligatures (via harfrust shaping).
