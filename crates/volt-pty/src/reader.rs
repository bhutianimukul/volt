//! PTY reader thread — blocks on `read()` from the master fd and sends bytes
//! to the parser thread via crossbeam-channel.
//!
//! Uses mio for non-blocking I/O with edge-triggered readiness notifications.
//! The channel is unbounded to prevent backpressure from stalling the reader.
