//! PTY reader thread — blocks on `read()` from the master fd and sends bytes
//! to the parser thread via crossbeam-channel.
//!
//! Uses mio for non-blocking I/O with edge-triggered readiness notifications.
//! The channel is unbounded to prevent backpressure from stalling the reader
//! (which would block the shell when the kernel buffer fills).

use std::io;
use std::os::fd::RawFd;
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender};
use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};

/// Messages sent from the reader thread to the consumer.
#[derive(Debug)]
pub enum PtyRead {
    /// A chunk of bytes read from the PTY.
    Data(Vec<u8>),
    /// The PTY has been closed (child exited or EOF).
    Closed,
    /// A read error occurred.
    Error(io::Error),
}

/// Read buffer size. 64KB matches common kernel pipe buffer size on macOS.
const READ_BUF_SIZE: usize = 64 * 1024;

/// Token for the PTY fd in the mio poll.
const PTY_TOKEN: Token = Token(0);
/// Token for the wakeup pipe (shutdown signal).
const WAKE_TOKEN: Token = Token(1);

/// A PTY reader that runs on a dedicated thread.
///
/// Reads from the master fd and sends byte chunks over a crossbeam channel.
/// Shutdown is signaled via a wakeup pipe to interrupt the poll.
pub struct PtyReader {
    /// Channel receiver — consumer reads `PtyRead` messages from here.
    rx: Receiver<PtyRead>,
    /// Write end of the wakeup pipe — write a byte to signal shutdown.
    wake_write: RawFd,
    /// Reader thread handle.
    thread: Option<JoinHandle<()>>,
}

impl PtyReader {
    /// Spawn a reader thread for the given PTY master fd.
    ///
    /// Returns the `PtyReader` handle. The consumer should read from `rx()`
    /// to receive byte chunks.
    pub fn spawn(pty_fd: RawFd) -> Result<Self, io::Error> {
        let (tx, rx) = crossbeam_channel::unbounded::<PtyRead>();

        // Create a pipe for shutdown signaling
        let mut pipe_fds = [0i32; 2];
        // SAFETY: pipe() creates a pair of file descriptors.
        let ret = unsafe { libc::pipe(pipe_fds.as_mut_ptr()) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }
        let wake_read = pipe_fds[0];
        let wake_write = pipe_fds[1];

        // Set the pipe read end to non-blocking
        // SAFETY: fcntl on valid fds we just created.
        unsafe {
            let flags = libc::fcntl(wake_read, libc::F_GETFL);
            libc::fcntl(wake_read, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }

        let thread = thread::Builder::new()
            .name("pty-reader".into())
            .spawn(move || {
                reader_loop(pty_fd, wake_read, tx);
                // Clean up the read end of the wake pipe
                // SAFETY: closing a valid fd we own.
                unsafe {
                    libc::close(wake_read);
                }
            })?;

        Ok(Self {
            rx,
            wake_write,
            thread: Some(thread),
        })
    }

    /// Get the receiver for PTY read messages.
    pub fn rx(&self) -> &Receiver<PtyRead> {
        &self.rx
    }

    /// Signal the reader thread to shut down and wait for it to exit.
    pub fn shutdown(mut self) {
        self.signal_shutdown();
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }

    fn signal_shutdown(&self) {
        // Write a byte to the wake pipe to interrupt the poll
        // SAFETY: writing a single byte to a valid pipe fd.
        unsafe {
            libc::write(self.wake_write, [1u8].as_ptr() as *const libc::c_void, 1);
        }
    }
}

impl Drop for PtyReader {
    fn drop(&mut self) {
        self.signal_shutdown();
        // Close the write end of the wake pipe
        // SAFETY: closing a valid fd we own.
        unsafe {
            libc::close(self.wake_write);
        }
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

/// The reader loop running on the dedicated thread.
fn reader_loop(pty_fd: RawFd, wake_fd: RawFd, tx: Sender<PtyRead>) {
    let mut poll = match Poll::new() {
        Ok(p) => p,
        Err(e) => {
            let _ = tx.send(PtyRead::Error(e));
            return;
        }
    };

    let mut events = Events::with_capacity(2);

    // Register the PTY fd for readable events
    // SAFETY: pty_fd is valid and in non-blocking mode (set during Pty::spawn).
    let mut pty_source = SourceFd(&pty_fd);
    if let Err(e) = poll
        .registry()
        .register(&mut pty_source, PTY_TOKEN, Interest::READABLE)
    {
        let _ = tx.send(PtyRead::Error(e));
        return;
    }

    // Register the wake pipe for shutdown notification
    let mut wake_source = SourceFd(&wake_fd);
    if let Err(e) = poll
        .registry()
        .register(&mut wake_source, WAKE_TOKEN, Interest::READABLE)
    {
        let _ = tx.send(PtyRead::Error(e));
        return;
    }

    let mut buf = vec![0u8; READ_BUF_SIZE];

    loop {
        if let Err(e) = poll.poll(&mut events, None) {
            if e.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            let _ = tx.send(PtyRead::Error(e));
            return;
        }

        for event in events.iter() {
            match event.token() {
                WAKE_TOKEN => {
                    // Shutdown signal received
                    return;
                }
                PTY_TOKEN => {
                    // Read all available data (edge-triggered: drain completely)
                    loop {
                        // SAFETY: reading from a valid non-blocking fd into our buffer.
                        let n = unsafe {
                            libc::read(pty_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len())
                        };

                        if n > 0 {
                            let data = buf[..n as usize].to_vec();
                            if tx.send(PtyRead::Data(data)).is_err() {
                                // Consumer dropped — shut down
                                return;
                            }
                        } else if n == 0 {
                            // EOF — child closed the PTY
                            let _ = tx.send(PtyRead::Closed);
                            return;
                        } else {
                            let err = io::Error::last_os_error();
                            match err.kind() {
                                io::ErrorKind::WouldBlock => break, // No more data right now
                                io::ErrorKind::Interrupted => continue,
                                _ => {
                                    // EIO typically means the child exited
                                    if err.raw_os_error() == Some(libc::EIO) {
                                        let _ = tx.send(PtyRead::Closed);
                                    } else {
                                        let _ = tx.send(PtyRead::Error(err));
                                    }
                                    return;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PtySize;
    use crate::pty::Pty;
    use std::time::Duration;

    #[test]
    fn reader_receives_output() {
        let pty = Pty::spawn(
            Some("/bin/sh"),
            &[],
            None,
            PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            },
        )
        .expect("spawn");

        let reader = PtyReader::spawn(pty.as_raw_fd()).expect("spawn reader");

        // Send a command that produces output
        pty.write_all(b"echo volt-test-marker\n").expect("write");

        // Read until we see our marker or timeout
        let mut found = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while std::time::Instant::now() < deadline {
            match reader.rx().recv_timeout(Duration::from_millis(100)) {
                Ok(PtyRead::Data(data)) => {
                    if String::from_utf8_lossy(&data).contains("volt-test-marker") {
                        found = true;
                        break;
                    }
                }
                Ok(PtyRead::Closed) => break,
                Ok(PtyRead::Error(_)) => break,
                Err(_) => continue,
            }
        }
        assert!(found, "should have received echo output");

        // Send exit to clean up the shell
        let _ = pty.write_all(b"exit\n");
        reader.shutdown();
    }
}
