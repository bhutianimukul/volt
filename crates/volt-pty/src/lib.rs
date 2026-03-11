//! volt-pty: PTY creation, I/O, and signal handling.
//!
//! Uses raw `forkpty(3)` via libc for macOS PTY management. Each terminal pane
//! gets a dedicated PTY with a reader thread that posts bytes to the parser
//! thread via `crossbeam-channel` (unbounded, to avoid backpressure stalling).

pub mod pty;
pub mod reader;
pub mod signal;

/// Configuration for spawning a new PTY.
pub struct PtyConfig {
    /// Shell to execute (e.g., "/bin/zsh"). If None, uses user's login shell.
    pub shell: Option<String>,
    /// Additional environment variables.
    pub env: Vec<(String, String)>,
    /// Initial working directory.
    pub working_dir: Option<std::path::PathBuf>,
    /// Initial terminal size.
    pub size: PtySize,
}

/// PTY dimensions.
#[derive(Debug, Clone, Copy)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}
