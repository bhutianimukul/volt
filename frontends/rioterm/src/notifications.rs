//! macOS notification support for long-running command completion.

use std::time::{Duration, Instant};

const DEFAULT_THRESHOLD: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct NotificationManager {
    threshold: Duration,
    pending_commands: Vec<PendingCommand>,
    enabled: bool,
}

#[derive(Debug)]
struct PendingCommand {
    command: String,
    started_at: Instant,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            threshold: DEFAULT_THRESHOLD,
            pending_commands: Vec::new(),
            enabled: true,
        }
    }

    pub fn set_threshold(&mut self, seconds: u64) {
        self.threshold = Duration::from_secs(seconds);
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Track a command that just started executing
    pub fn command_started(&mut self, command: String) {
        self.pending_commands.push(PendingCommand {
            command,
            started_at: Instant::now(),
        });
    }

    /// Called when a command finishes. Sends notification if it ran longer than threshold.
    pub fn command_finished(&mut self, exit_code: i32) {
        if !self.enabled {
            self.pending_commands.pop();
            return;
        }

        if let Some(pending) = self.pending_commands.pop() {
            let elapsed = pending.started_at.elapsed();
            if elapsed >= self.threshold {
                let status = if exit_code == 0 {
                    "completed"
                } else {
                    "failed"
                };
                let title = format!("Command {}", status);
                let body = format!(
                    "{}\n{} in {:.1}s (exit {})",
                    pending.command,
                    status,
                    elapsed.as_secs_f64(),
                    exit_code
                );
                send_notification(&title, &body, exit_code != 0);
            }
        }
    }

    /// Clear all pending commands (e.g., on tab close)
    pub fn clear(&mut self) {
        self.pending_commands.clear();
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Send a macOS notification using NSUserNotificationCenter or osascript fallback
#[cfg(target_os = "macos")]
fn send_notification(title: &str, body: &str, is_error: bool) {
    use objc::runtime::{Class, Object};
    use objc::{msg_send, sel, sel_impl};
    use std::ffi::CString;

    unsafe {
        // Try NSUserNotificationCenter
        let center_class = Class::get("NSUserNotificationCenter");
        let notification_class = Class::get("NSUserNotification");

        if let (Some(center_cls), Some(notif_cls)) = (center_class, notification_class) {
            let center: *mut Object =
                msg_send![center_cls, defaultUserNotificationCenter];
            let notification: *mut Object = msg_send![notif_cls, new];

            let ns_string_class = Class::get("NSString").unwrap();

            let title_cstr = CString::new(title).unwrap_or_default();
            let title_ns: *mut Object =
                msg_send![ns_string_class, stringWithUTF8String: title_cstr.as_ptr()];
            let _: () = msg_send![notification, setTitle: title_ns];

            let body_cstr = CString::new(body).unwrap_or_default();
            let body_ns: *mut Object =
                msg_send![ns_string_class, stringWithUTF8String: body_cstr.as_ptr()];
            let _: () = msg_send![notification, setInformativeText: body_ns];

            // Sound for errors
            if is_error {
                let sound_name: *mut Object = msg_send![ns_string_class,
                    stringWithUTF8String: b"Basso\0".as_ptr()];
                let _: () = msg_send![notification, setSoundName: sound_name];
            } else {
                let sound_name: *mut Object = msg_send![ns_string_class,
                    stringWithUTF8String: b"default\0".as_ptr()];
                let _: () = msg_send![notification, setSoundName: sound_name];
            }

            let _: () = msg_send![center, deliverNotification: notification];
        } else {
            // Fallback to osascript
            let escaped_title = title.replace('"', r#"\""#);
            let escaped_body = body.replace('"', r#"\""#);
            let _ = std::process::Command::new("osascript")
                .args([
                    "-e",
                    &format!(
                        r#"display notification "{}" with title "{}""#,
                        escaped_body, escaped_title
                    ),
                ])
                .spawn();
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn send_notification(title: &str, body: &str, _is_error: bool) {
    // Try notify-send on Linux
    let _ = std::process::Command::new("notify-send")
        .args([title, body])
        .spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_manager_below_threshold() {
        let mut mgr = NotificationManager::new();
        mgr.set_enabled(false); // Don't actually send
        mgr.command_started("ls".to_string());
        // Immediately finish — below threshold
        mgr.command_finished(0);
        assert!(mgr.pending_commands.is_empty());
    }

    #[test]
    fn test_notification_manager_disabled() {
        let mut mgr = NotificationManager::new();
        mgr.set_enabled(false);
        mgr.command_started("sleep 100".to_string());
        mgr.command_finished(0); // Should not crash even when disabled
    }

    #[test]
    fn test_set_threshold() {
        let mut mgr = NotificationManager::new();
        mgr.set_threshold(30);
        assert_eq!(mgr.threshold, Duration::from_secs(30));
    }

    #[test]
    fn test_clear_pending() {
        let mut mgr = NotificationManager::new();
        mgr.command_started("cargo build".to_string());
        mgr.command_started("cargo test".to_string());
        assert_eq!(mgr.pending_commands.len(), 2);
        mgr.clear();
        assert!(mgr.pending_commands.is_empty());
    }

    #[test]
    fn test_default_trait() {
        let mgr = NotificationManager::default();
        assert!(mgr.enabled);
        assert_eq!(mgr.threshold, Duration::from_secs(10));
        assert!(mgr.pending_commands.is_empty());
    }
}
