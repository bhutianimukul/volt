//! Audit logging — structured, append-only log of significant terminal events.
//! Used for security auditing and debugging.

use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    pub timestamp: u64, // Unix epoch seconds
    pub event: AuditEvent,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum AuditEvent {
    #[serde(rename = "session_start")]
    SessionStart { shell: String, working_dir: String },
    #[serde(rename = "session_end")]
    SessionEnd { duration_secs: u64 },
    #[serde(rename = "command_executed")]
    CommandExecuted {
        command: String,
        working_dir: String,
    },
    #[serde(rename = "command_completed")]
    CommandCompleted {
        command: String,
        exit_code: i32,
        duration_ms: u64,
    },
    #[serde(rename = "destructive_command_blocked")]
    DestructiveCommandBlocked { command: String, reason: String },
    #[serde(rename = "destructive_command_allowed")]
    DestructiveCommandAllowed { command: String },
    #[serde(rename = "secret_detected")]
    SecretDetected {
        pattern_type: String,
        context: String,
    },
    #[serde(rename = "connection_opened")]
    ConnectionOpened {
        connection_type: String,
        target: String,
    },
    #[serde(rename = "file_modified")]
    FileModified { path: String, operation: String },
    #[serde(rename = "checkpoint_created")]
    CheckpointCreated {
        checkpoint_id: usize,
        command: String,
    },
    #[serde(rename = "undo_performed")]
    UndoPerformed {
        checkpoint_id: usize,
        files_restored: usize,
    },
    #[serde(rename = "config_changed")]
    ConfigChanged {
        key: String,
        old_value: String,
        new_value: String,
    },
}

pub struct AuditLogger {
    session_id: String,
    log_file: Option<Mutex<std::fs::File>>,
    enabled: bool,
    entries_count: u64,
}

impl AuditLogger {
    pub fn new(enabled: bool) -> Self {
        let session_id = generate_session_id();
        let log_file = if enabled {
            Self::open_log_file().ok().map(Mutex::new)
        } else {
            None
        };

        Self {
            session_id,
            log_file,
            enabled,
            entries_count: 0,
        }
    }

    fn open_log_file() -> Result<std::fs::File, std::io::Error> {
        let path = audit_log_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
    }

    /// Log an audit event
    pub fn log(&mut self, event: AuditEvent) {
        if !self.enabled {
            return;
        }

        let entry = AuditEntry {
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            event,
            session_id: self.session_id.clone(),
        };

        self.entries_count += 1;

        // Write to file
        if let Some(ref file_mutex) = self.log_file {
            if let Ok(mut file) = file_mutex.lock() {
                if let Ok(json) = serde_json::to_string(&entry) {
                    let _ = writeln!(file, "{}", json);
                }
            }
        }

        // Also log via tracing at debug level
        tracing::debug!("audit: {:?}", entry);
    }

    /// Log a command execution
    pub fn log_command(&mut self, command: &str, working_dir: &str) {
        self.log(AuditEvent::CommandExecuted {
            command: command.to_string(),
            working_dir: working_dir.to_string(),
        });
    }

    /// Log a destructive command that was blocked
    pub fn log_blocked(&mut self, command: &str, reason: &str) {
        self.log(AuditEvent::DestructiveCommandBlocked {
            command: command.to_string(),
            reason: reason.to_string(),
        });
    }

    /// Log a secret detection
    pub fn log_secret(&mut self, pattern_type: &str, context: &str) {
        self.log(AuditEvent::SecretDetected {
            pattern_type: pattern_type.to_string(),
            context: context.to_string(),
        });
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn entries_count(&self) -> u64 {
        self.entries_count
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled && self.log_file.is_none() {
            self.log_file = Self::open_log_file().ok().map(Mutex::new);
        }
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(false) // Disabled by default
    }
}

fn audit_log_path() -> PathBuf {
    rio_backend::config::config_dir_path()
        .join("log")
        .join("audit.jsonl")
}

fn generate_session_id() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("volt-{}-{}", now.as_secs(), now.subsec_nanos() % 10000)
}

/// Read recent audit entries from the log file
pub fn read_recent_entries(count: usize) -> Vec<String> {
    let path = audit_log_path();
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => content
            .lines()
            .rev()
            .take(count)
            .map(|s| s.to_string())
            .collect(),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_generation() {
        let id = generate_session_id();
        assert!(id.starts_with("volt-"));
        assert!(id.len() > 10);
    }

    #[test]
    fn test_audit_logger_disabled() {
        let mut logger = AuditLogger::new(false);
        logger.log_command("ls", "/home");
        assert_eq!(logger.entries_count(), 0);
    }

    #[test]
    fn test_audit_logger_enabled() {
        let mut logger = AuditLogger::new(true);
        logger.log_command("ls -la", "/home");
        assert_eq!(logger.entries_count(), 1);
        logger.log_blocked("rm -rf /", "dangerous");
        assert_eq!(logger.entries_count(), 2);
    }

    #[test]
    fn test_audit_entry_serialization() {
        let entry = AuditEntry {
            timestamp: 1700000000,
            event: AuditEvent::CommandExecuted {
                command: "cargo test".into(),
                working_dir: "/projects/volt".into(),
            },
            session_id: "volt-123-456".into(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("command_executed"));
        assert!(json.contains("cargo test"));
    }
}
