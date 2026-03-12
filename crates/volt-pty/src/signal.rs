//! Signal handling — SIGCHLD for child process exit detection.
//!
//! Uses a self-pipe trick: the SIGCHLD handler writes a byte to a pipe,
//! and the consumer can poll/select on the read end to detect child exits.
//! We then call `waitpid(WNOHANG)` to reap the child and get its exit status.

use std::io;
use std::os::fd::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};

static SIGCHLD_INSTALLED: AtomicBool = AtomicBool::new(false);

/// Pipe write end for the SIGCHLD handler. Set once during install.
/// Using a raw static because signal handlers can only access atomics and raw fds.
static mut SIGCHLD_PIPE_WRITE: RawFd = -1;

/// Install a process-wide SIGCHLD handler that writes to the given pipe.
///
/// This should be called once during application startup. The read end of
/// the pipe can be polled to detect child process exits.
///
/// Returns the (read_fd, write_fd) pipe pair if newly installed, or an error.
pub fn install_sigchld_handler() -> Result<RawFd, io::Error> {
    if SIGCHLD_INSTALLED.swap(true, Ordering::SeqCst) {
        // Already installed — return the existing read fd
        // This is a programming error but we handle it gracefully
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "SIGCHLD handler already installed",
        ));
    }

    let mut pipe_fds = [0i32; 2];
    // SAFETY: pipe() creates a valid pair of fds.
    let ret = unsafe { libc::pipe(pipe_fds.as_mut_ptr()) };
    if ret < 0 {
        SIGCHLD_INSTALLED.store(false, Ordering::SeqCst);
        return Err(io::Error::last_os_error());
    }
    let read_fd = pipe_fds[0];
    let write_fd = pipe_fds[1];

    // Set both ends to non-blocking and close-on-exec
    // SAFETY: fcntl on valid fds we just created.
    unsafe {
        for fd in [read_fd, write_fd] {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
        }
    }

    // Store the write end for the signal handler
    // SAFETY: This is set once before the signal handler is installed,
    // and the handler only reads it.
    unsafe {
        SIGCHLD_PIPE_WRITE = write_fd;
    }

    // Install the signal handler
    // SAFETY: sigaction with a valid handler function.
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = sigchld_handler as libc::sighandler_t;
        sa.sa_flags = libc::SA_NOCLDSTOP | libc::SA_RESTART;
        libc::sigemptyset(&mut sa.sa_mask);

        let ret = libc::sigaction(libc::SIGCHLD, &sa, std::ptr::null_mut());
        if ret < 0 {
            libc::close(read_fd);
            libc::close(write_fd);
            SIGCHLD_INSTALLED.store(false, Ordering::SeqCst);
            return Err(io::Error::last_os_error());
        }
    }

    Ok(read_fd)
}

/// The SIGCHLD signal handler. Async-signal-safe: only writes one byte to the pipe.
///
/// SAFETY: This function is called by the kernel in signal context. It only
/// uses `write()` (async-signal-safe) on a pre-initialized fd.
extern "C" fn sigchld_handler(_sig: libc::c_int) {
    // SAFETY: SIGCHLD_PIPE_WRITE was set before the handler was installed
    // and never modified after. write() is async-signal-safe.
    unsafe {
        let _ = libc::write(SIGCHLD_PIPE_WRITE, [1u8].as_ptr() as *const libc::c_void, 1);
    }
}

/// Wait for a specific child process (non-blocking).
///
/// Returns `Some(exit_status)` if the child has exited, `None` if still running.
/// Reaps the child (prevents zombies).
pub fn try_wait_child(pid: libc::pid_t) -> Option<ChildExit> {
    let mut status: libc::c_int = 0;
    // SAFETY: waitpid with WNOHANG on a valid pid.
    let ret = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };

    if ret <= 0 {
        // ret == 0: child still running
        // ret < 0: error (e.g., no such child)
        return None;
    }

    if libc::WIFEXITED(status) {
        Some(ChildExit::Normal(libc::WEXITSTATUS(status)))
    } else if libc::WIFSIGNALED(status) {
        Some(ChildExit::Signal(libc::WTERMSIG(status)))
    } else {
        None
    }
}

/// Drain the SIGCHLD notification pipe (consume all pending bytes).
///
/// Call this after receiving a readable event on the SIGCHLD pipe,
/// before calling `try_wait_child()`.
pub fn drain_sigchld_pipe(read_fd: RawFd) {
    let mut buf = [0u8; 64];
    loop {
        // SAFETY: reading from a valid non-blocking fd into our buffer.
        let n = unsafe { libc::read(read_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n <= 0 {
            break;
        }
    }
}

/// How a child process exited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildExit {
    /// Normal exit with status code.
    Normal(i32),
    /// Killed by signal.
    Signal(i32),
}

impl ChildExit {
    /// Whether the child exited successfully (status 0).
    pub fn success(self) -> bool {
        matches!(self, Self::Normal(0))
    }

    /// Get the exit code (or signal number negated).
    pub fn code(self) -> i32 {
        match self {
            Self::Normal(code) => code,
            Self::Signal(sig) => -sig,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_exit_success() {
        assert!(ChildExit::Normal(0).success());
        assert!(!ChildExit::Normal(1).success());
        assert!(!ChildExit::Signal(9).success());
    }

    #[test]
    fn child_exit_code() {
        assert_eq!(ChildExit::Normal(42).code(), 42);
        assert_eq!(ChildExit::Signal(9).code(), -9);
    }

    #[test]
    fn try_wait_nonexistent_pid() {
        // Waiting on an invalid pid should return None
        assert!(try_wait_child(99999999).is_none());
    }
}
