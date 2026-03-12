//! volt-pty: PTY creation, I/O, and signal handling.
//!
//! Uses raw `forkpty(3)` via libc for macOS PTY management. Each terminal pane
//! gets a dedicated PTY with a reader thread that posts bytes to the parser
//! thread via `crossbeam-channel` (unbounded, to avoid backpressure stalling).
//!
//! # Usage
//!
//! ```no_run
//! use volt_pty::{PtyHandle, PtyConfig, PtySize};
//!
//! let config = PtyConfig {
//!     shell: None, // Use login shell
//!     env: vec![],
//!     working_dir: None,
//!     size: PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 },
//! };
//!
//! let handle = PtyHandle::spawn(config).expect("spawn PTY");
//!
//! // Write input to the shell
//! handle.write(b"ls\n").expect("write");
//!
//! // Read output from the reader channel
//! while let Ok(msg) = handle.rx().recv() {
//!     match msg {
//!         volt_pty::reader::PtyRead::Data(bytes) => { /* process bytes */ }
//!         volt_pty::reader::PtyRead::Closed => break,
//!         volt_pty::reader::PtyRead::Error(e) => { eprintln!("error: {e}"); break; }
//!     }
//! }
//! ```

pub mod pty;
pub mod reader;
pub mod signal;

use crossbeam_channel::Receiver;

use pty::{Pty, PtyError};
use reader::{PtyRead, PtyReader};

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

/// High-level PTY handle that owns the PTY and its reader thread.
///
/// This is the main API for volt-app. It manages the lifecycle of the
/// PTY connection: spawn, read, write, resize, and shutdown.
pub struct PtyHandle {
    /// The PTY (master fd + child pid).
    pty: Pty,
    /// The reader thread + channel.
    reader: Option<PtyReader>,
}

impl PtyHandle {
    /// Spawn a new PTY with the given configuration.
    ///
    /// Creates a new pseudo-terminal, forks a child process running the shell,
    /// and starts a reader thread to send output bytes over a channel.
    pub fn spawn(config: PtyConfig) -> Result<Self, PtyError> {
        let pty = Pty::spawn(
            config.shell.as_deref(),
            &config.env,
            config.working_dir.as_deref(),
            config.size,
        )?;

        let reader = PtyReader::spawn(pty.as_raw_fd()).map_err(PtyError::Io)?;

        Ok(Self {
            pty,
            reader: Some(reader),
        })
    }

    /// Write bytes to the PTY (sends input to the shell).
    pub fn write(&self, data: &[u8]) -> Result<(), PtyError> {
        self.pty.write_all(data)
    }

    /// Get the receiver channel for PTY output.
    ///
    /// The reader thread sends `PtyRead::Data(bytes)` for output,
    /// `PtyRead::Closed` when the child exits, and `PtyRead::Error` on failure.
    pub fn rx(&self) -> &Receiver<PtyRead> {
        // Reader is always Some while PtyHandle is alive (only taken on drop)
        self.reader.as_ref().expect("reader exists").rx()
    }

    /// Resize the PTY. Sends TIOCSWINSZ to deliver SIGWINCH to the shell.
    pub fn resize(&self, size: PtySize) -> Result<(), PtyError> {
        self.pty.resize(size)
    }

    /// Get the child process ID.
    pub fn child_pid(&self) -> libc::pid_t {
        self.pty.child_pid()
    }

    /// Check if the child process has exited (non-blocking).
    ///
    /// Returns `Some(ChildExit)` if exited, `None` if still running.
    pub fn try_wait(&self) -> Option<signal::ChildExit> {
        signal::try_wait_child(self.pty.child_pid())
    }

    /// Shut down the PTY handle, stopping the reader thread.
    /// The child process is killed if still running.
    pub fn shutdown(mut self) {
        self.shutdown_inner();
    }

    fn shutdown_inner(&mut self) {
        if let Some(reader) = self.reader.take() {
            reader.shutdown();
        }
    }
}

impl Drop for PtyHandle {
    fn drop(&mut self) {
        self.shutdown_inner();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn test_config() -> PtyConfig {
        PtyConfig {
            shell: Some("/bin/sh".into()),
            env: vec![],
            working_dir: None,
            size: PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            },
        }
    }

    #[test]
    fn spawn_and_communicate() {
        let handle = PtyHandle::spawn(test_config()).expect("spawn");

        // Write a command
        handle.write(b"echo VOLT_ROUNDTRIP\n").expect("write");

        // Read until we find our marker
        let mut found = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while std::time::Instant::now() < deadline {
            match handle.rx().recv_timeout(Duration::from_millis(100)) {
                Ok(PtyRead::Data(data)) => {
                    if String::from_utf8_lossy(&data).contains("VOLT_ROUNDTRIP") {
                        found = true;
                        break;
                    }
                }
                Ok(PtyRead::Closed) => break,
                Ok(PtyRead::Error(_)) => break,
                Err(_) => continue,
            }
        }
        assert!(found, "should receive echo output from PTY");

        // Exit the shell
        let _ = handle.write(b"exit\n");

        // Wait for close
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            match handle.rx().recv_timeout(Duration::from_millis(100)) {
                Ok(PtyRead::Closed) => break,
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                _ => continue,
            }
        }
    }

    #[test]
    fn resize_succeeds() {
        let handle = PtyHandle::spawn(test_config()).expect("spawn");
        handle
            .resize(PtySize {
                rows: 50,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("resize should succeed");

        let _ = handle.write(b"exit\n");
    }

    #[test]
    fn child_pid_is_valid() {
        let handle = PtyHandle::spawn(test_config()).expect("spawn");
        assert!(handle.child_pid() > 0);
        let _ = handle.write(b"exit\n");
    }

    #[test]
    fn detect_child_exit() {
        let handle = PtyHandle::spawn(test_config()).expect("spawn");
        handle.write(b"exit 0\n").expect("write");

        // Wait for exit
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut exited = false;
        while std::time::Instant::now() < deadline {
            if let Some(exit) = handle.try_wait() {
                assert!(exit.success(), "child should exit with code 0");
                exited = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(exited, "child should have exited");
    }
}
