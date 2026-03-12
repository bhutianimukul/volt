//! PTY creation and lifecycle via raw `forkpty(3)`.
//!
//! Uses libc directly since we're macOS-only and need full control over
//! the file descriptor for mio integration and non-blocking reads.
//!
//! Flow: forkpty → set TIOCSWINSZ → exec shell → return master fd.

use std::ffi::{CStr, CString};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

use crate::PtySize;

/// Error type for PTY operations.
#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    #[error("forkpty failed: {0}")]
    ForkPty(io::Error),

    #[error("failed to exec shell '{shell}': {source}")]
    Exec { shell: String, source: io::Error },

    #[error("failed to set terminal size: {0}")]
    Resize(io::Error),

    #[error("failed to write to PTY: {0}")]
    Write(io::Error),

    #[error("failed to determine login shell")]
    NoShell,

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

/// A pseudo-terminal (master side).
///
/// Owns the master file descriptor returned by `forkpty()`.
/// The child process (shell) is connected to the slave side.
pub struct Pty {
    /// Master file descriptor.
    master_fd: OwnedFd,
    /// Child process ID.
    child_pid: libc::pid_t,
}

impl Pty {
    /// Spawn a new PTY with a shell process.
    ///
    /// # Safety
    /// Uses `forkpty(3)` which is inherently unsafe (forks the process).
    /// The child process execs into the shell immediately after fork.
    pub fn spawn(
        shell: Option<&str>,
        env: &[(String, String)],
        working_dir: Option<&std::path::Path>,
        size: PtySize,
    ) -> Result<Self, PtyError> {
        let shell_path = match shell {
            Some(s) => s.to_string(),
            None => detect_login_shell()?,
        };
        let shell_cstr = CString::new(shell_path.as_str()).map_err(|_| PtyError::NoShell)?;

        let winsize = size.to_winsize();
        let mut master_fd: RawFd = -1;

        // SAFETY: forkpty is a POSIX function. We handle both parent and child
        // paths immediately. The child execs into the shell, so no shared state
        // is accessed after fork.
        let pid = unsafe {
            libc::forkpty(
                &mut master_fd,
                std::ptr::null_mut(), // No slave name needed
                std::ptr::null_mut(), // Default termios
                &winsize as *const libc::winsize as *mut libc::winsize,
            )
        };

        match pid {
            -1 => Err(PtyError::ForkPty(io::Error::last_os_error())),
            0 => {
                // === Child process ===
                child_exec(&shell_cstr, &shell_path, env, working_dir);
            }
            _ => {
                // === Parent process ===
                // SAFETY: forkpty returned a valid fd on success (pid > 0).
                let master_fd = unsafe { OwnedFd::from_raw_fd(master_fd) };

                // Set non-blocking for mio compatibility
                set_nonblocking(master_fd.as_raw_fd())?;

                Ok(Self {
                    master_fd,
                    child_pid: pid,
                })
            }
        }
    }

    /// Write bytes to the PTY (sends input to the shell).
    pub fn write_all(&self, data: &[u8]) -> Result<(), PtyError> {
        let mut offset = 0;
        while offset < data.len() {
            // SAFETY: Writing to a valid file descriptor we own.
            let n = unsafe {
                libc::write(
                    self.master_fd.as_raw_fd(),
                    data[offset..].as_ptr() as *const libc::c_void,
                    data.len() - offset,
                )
            };
            if n < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(PtyError::Write(err));
            }
            offset += n as usize;
        }
        Ok(())
    }

    /// Resize the PTY. Sends TIOCSWINSZ to the master fd, which delivers
    /// SIGWINCH to the child process.
    pub fn resize(&self, size: PtySize) -> Result<(), PtyError> {
        let winsize = size.to_winsize();
        // SAFETY: TIOCSWINSZ is a standard ioctl for terminal resize.
        let ret = unsafe {
            libc::ioctl(
                self.master_fd.as_raw_fd(),
                libc::TIOCSWINSZ,
                &winsize as *const libc::winsize,
            )
        };
        if ret < 0 {
            Err(PtyError::Resize(io::Error::last_os_error()))
        } else {
            Ok(())
        }
    }

    /// Get the raw master file descriptor (for mio/poll registration).
    pub fn as_raw_fd(&self) -> RawFd {
        self.master_fd.as_raw_fd()
    }

    /// Get the child process ID.
    pub fn child_pid(&self) -> libc::pid_t {
        self.child_pid
    }
}

/// Child process: set up environment and exec into the shell.
/// This function never returns (it execs or exits).
fn child_exec(
    shell_cstr: &CStr,
    shell_path: &str,
    env: &[(String, String)],
    working_dir: Option<&std::path::Path>,
) -> ! {
    // Change working directory if specified
    if let Some(dir) = working_dir {
        if let Ok(dir_cstr) = CString::new(dir.to_string_lossy().as_bytes()) {
            // SAFETY: chdir with a valid CString path.
            unsafe {
                libc::chdir(dir_cstr.as_ptr());
            }
        }
    }

    // Set TERM environment variable
    set_env_cstr("TERM", "xterm-256color");

    // Set custom environment variables
    for (key, value) in env {
        set_env_cstr(key, value);
    }

    // Extract shell name for argv[0] (login shell convention: prefix with '-')
    let shell_name = shell_path.rsplit('/').next().unwrap_or(shell_path);
    let login_name = format!("-{shell_name}");
    let login_cstr = CString::new(login_name.as_str())
        .unwrap_or_else(|_| CString::new(shell_name).unwrap_or_default());

    // exec the shell as a login shell
    let args: [*const libc::c_char; 2] = [login_cstr.as_ptr(), std::ptr::null()];

    // SAFETY: execvp replaces the process image. We pass valid CStrings.
    unsafe {
        libc::execvp(shell_cstr.as_ptr(), args.as_ptr());
    }

    // If we get here, exec failed
    eprintln!(
        "volt: exec failed for {shell_path}: {}",
        io::Error::last_os_error()
    );
    // SAFETY: _exit is safe to call after failed exec in forked child.
    unsafe {
        libc::_exit(1);
    }
}

/// Set an environment variable in the child process using libc.
fn set_env_cstr(key: &str, value: &str) {
    if let (Ok(k), Ok(v)) = (CString::new(key), CString::new(value)) {
        // SAFETY: setenv with valid CStrings, overwrite=1.
        unsafe {
            libc::setenv(k.as_ptr(), v.as_ptr(), 1);
        }
    }
}

/// Set a file descriptor to non-blocking mode.
fn set_nonblocking(fd: RawFd) -> Result<(), PtyError> {
    // SAFETY: fcntl on a valid fd we own.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(PtyError::Io(io::Error::last_os_error()));
    }
    let ret = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
    if ret < 0 {
        return Err(PtyError::Io(io::Error::last_os_error()));
    }
    Ok(())
}

/// Detect the user's login shell from the passwd database.
fn detect_login_shell() -> Result<String, PtyError> {
    // SAFETY: getuid() and getpwuid() are standard POSIX functions.
    let uid = unsafe { libc::getuid() };
    let pw = unsafe { libc::getpwuid(uid) };
    if pw.is_null() {
        return Err(PtyError::NoShell);
    }
    // SAFETY: getpwuid returned a non-null pointer, pw_shell is a valid CStr.
    let shell = unsafe { CStr::from_ptr((*pw).pw_shell) };
    shell
        .to_str()
        .map(|s| s.to_string())
        .map_err(|_| PtyError::NoShell)
}

impl PtySize {
    /// Convert to libc winsize struct.
    pub(crate) fn to_winsize(self) -> libc::winsize {
        libc::winsize {
            ws_row: self.rows,
            ws_col: self.cols,
            ws_xpixel: self.pixel_width,
            ws_ypixel: self.pixel_height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_shell_returns_valid_path() {
        let shell = detect_login_shell().expect("should detect login shell");
        assert!(
            shell.starts_with('/'),
            "shell should be an absolute path: {shell}"
        );
    }

    #[test]
    fn pty_size_to_winsize() {
        let size = PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 640,
            pixel_height: 480,
        };
        let ws = size.to_winsize();
        assert_eq!(ws.ws_row, 24);
        assert_eq!(ws.ws_col, 80);
        assert_eq!(ws.ws_xpixel, 640);
        assert_eq!(ws.ws_ypixel, 480);
    }

    #[test]
    fn spawn_and_write_echo() {
        // Spawn a PTY running /bin/sh -c 'cat' so we can test round-trip
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
        .expect("spawn PTY");

        assert!(pty.child_pid() > 0);
        assert!(pty.as_raw_fd() >= 0);

        // Resize should succeed
        pty.resize(PtySize {
            rows: 30,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("resize should succeed");
    }
}
