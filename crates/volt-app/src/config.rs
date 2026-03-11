//! TOML configuration with hot-reload.
//!
//! Double-buffer pattern: parse new config on change, atomic swap into active config.
//! Covers: fonts, colors, keybindings, behavior, profiles.
//!
//! Config file: ~/.config/volt/config.toml
