//! PTY creation and lifecycle via raw `forkpty(3)`.
//!
//! Reference: WezTerm's `portable-pty` for the forkpty pattern, but we go direct
//! since we're macOS-only and need full control over the file descriptor.
//!
//! Flow: forkpty → set TIOCSWINSZ → exec shell → return master fd.
