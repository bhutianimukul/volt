//! Tmux Control Center (CC) mode integration.
//!
//! When connected via `tmux -CC`, tmux sends structured control messages
//! instead of drawing to the terminal. Volt interprets these to map:
//! - tmux windows -> Volt tabs
//! - tmux panes -> Volt splits
//! - tmux sessions -> Volt session groups
//!
//! Protocol: tmux sends lines prefixed with %begin/%end/%error for responses,
//! and notifications like %session-changed, %window-add, %window-close, etc.

use std::collections::HashMap;

/// Represents a tmux session
#[derive(Debug, Clone)]
pub struct TmuxSession {
    pub id: String,
    pub name: String,
    pub windows: Vec<TmuxWindow>,
    pub created_at: u64,
    pub attached: bool,
}

/// Represents a tmux window (maps to a Volt tab)
#[derive(Debug, Clone)]
pub struct TmuxWindow {
    pub id: String,
    pub name: String,
    pub index: usize,
    pub panes: Vec<TmuxPane>,
    pub active: bool,
    pub layout: String,
}

/// Represents a tmux pane (maps to a Volt split pane)
#[derive(Debug, Clone)]
pub struct TmuxPane {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub active: bool,
    pub pid: u32,
    pub current_command: String,
    pub current_path: String,
}

/// Tmux CC mode notification types
#[derive(Debug, Clone)]
pub enum TmuxNotification {
    SessionChanged { session_id: String, name: String },
    SessionRenamed { name: String },
    WindowAdd { window_id: String },
    WindowClose { window_id: String },
    WindowRenamed { window_id: String, name: String },
    PaneChanged { pane_id: String },
    LayoutChange { window_id: String, layout: String },
    Output { pane_id: String, data: Vec<u8> },
    Exit,
    Error(String),
}

/// Parse a tmux CC notification line
pub fn parse_notification(line: &str) -> Option<TmuxNotification> {
    let trimmed = line.trim();

    if trimmed.starts_with("%session-changed") {
        let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
        if parts.len() >= 3 {
            return Some(TmuxNotification::SessionChanged {
                session_id: parts[1].trim_start_matches('$').to_string(),
                name: parts[2].to_string(),
            });
        }
    }

    if trimmed.starts_with("%session-renamed") {
        let name = trimmed.strip_prefix("%session-renamed ")?.to_string();
        return Some(TmuxNotification::SessionRenamed { name });
    }

    if trimmed.starts_with("%window-add") {
        let id = trimmed
            .strip_prefix("%window-add ")?
            .trim_start_matches('@')
            .to_string();
        return Some(TmuxNotification::WindowAdd { window_id: id });
    }

    if trimmed.starts_with("%window-close") {
        let id = trimmed
            .strip_prefix("%window-close ")?
            .trim_start_matches('@')
            .to_string();
        return Some(TmuxNotification::WindowClose { window_id: id });
    }

    if trimmed.starts_with("%window-renamed") {
        let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
        if parts.len() >= 3 {
            return Some(TmuxNotification::WindowRenamed {
                window_id: parts[1].trim_start_matches('@').to_string(),
                name: parts[2].to_string(),
            });
        }
    }

    if trimmed.starts_with("%layout-change") {
        let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
        if parts.len() >= 3 {
            return Some(TmuxNotification::LayoutChange {
                window_id: parts[1].trim_start_matches('@').to_string(),
                layout: parts[2].to_string(),
            });
        }
    }

    if trimmed == "%exit" {
        return Some(TmuxNotification::Exit);
    }

    if trimmed.starts_with("%error") {
        let msg = trimmed
            .strip_prefix("%error ")
            .unwrap_or(trimmed)
            .to_string();
        return Some(TmuxNotification::Error(msg));
    }

    None
}

/// Tmux CC mode controller -- manages the connection state
#[derive(Debug)]
pub struct TmuxController {
    pub sessions: Vec<TmuxSession>,
    pub active_session: Option<String>,
    pub windows: HashMap<String, TmuxWindow>,
    pub is_connected: bool,
    pending_response: Vec<String>,
    in_response_block: bool,
}

impl TmuxController {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            active_session: None,
            windows: HashMap::new(),
            is_connected: false,
            pending_response: Vec::new(),
            in_response_block: false,
        }
    }

    /// Process a line of tmux CC output
    pub fn process_line(&mut self, line: &str) -> Option<TmuxNotification> {
        let trimmed = line.trim();

        // Handle response blocks
        if trimmed.starts_with("%begin") {
            self.in_response_block = true;
            self.pending_response.clear();
            return None;
        }
        if trimmed.starts_with("%end") {
            self.in_response_block = false;
            // Process the accumulated response
            self.process_response();
            return None;
        }
        if self.in_response_block {
            self.pending_response.push(trimmed.to_string());
            return None;
        }

        // Handle notifications
        if let Some(notification) = parse_notification(trimmed) {
            self.handle_notification(&notification);
            return Some(notification);
        }

        None
    }

    fn handle_notification(&mut self, notification: &TmuxNotification) {
        match notification {
            TmuxNotification::SessionChanged {
                session_id, name, ..
            } => {
                self.active_session = Some(session_id.clone());
                tracing::info!("tmux session changed: {} ({})", name, session_id);
            }
            TmuxNotification::WindowAdd { window_id } => {
                self.windows.insert(
                    window_id.clone(),
                    TmuxWindow {
                        id: window_id.clone(),
                        name: String::new(),
                        index: self.windows.len(),
                        panes: Vec::new(),
                        active: false,
                        layout: String::new(),
                    },
                );
                tracing::info!("tmux window added: {}", window_id);
            }
            TmuxNotification::WindowClose { window_id } => {
                self.windows.remove(window_id);
                tracing::info!("tmux window closed: {}", window_id);
            }
            TmuxNotification::WindowRenamed { window_id, name } => {
                if let Some(win) = self.windows.get_mut(window_id) {
                    win.name = name.clone();
                }
            }
            TmuxNotification::LayoutChange { window_id, layout } => {
                if let Some(win) = self.windows.get_mut(window_id) {
                    win.layout = layout.clone();
                }
            }
            TmuxNotification::Exit => {
                self.is_connected = false;
                tracing::info!("tmux CC mode disconnected");
            }
            _ => {}
        }
    }

    fn process_response(&mut self) {
        // Parse list-sessions, list-windows, list-panes responses
        for line in &self.pending_response {
            tracing::debug!("tmux response: {}", line);
        }
    }

    /// Generate the command to start tmux CC mode
    pub fn connect_command(session_name: Option<&str>) -> String {
        match session_name {
            Some(name) => format!("tmux -CC attach -t {}", name),
            None => "tmux -CC new-session".to_string(),
        }
    }

    /// Generate tmux commands to send through CC mode
    pub fn send_command(command: &str) -> Vec<u8> {
        format!("{}\n", command).into_bytes()
    }

    /// List available tmux sessions (runs tmux list-sessions externally)
    pub fn list_sessions() -> Vec<(String, String, bool)> {
        let output = std::process::Command::new("tmux")
            .args([
                "list-sessions",
                "-F",
                "#{session_id}:#{session_name}:#{session_attached}",
            ])
            .output();

        match output {
            Ok(out) => String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        Some((
                            parts[0].to_string(),
                            parts[1].to_string(),
                            parts[2] == "1",
                        ))
                    } else {
                        None
                    }
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }
}

impl Default for TmuxController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_session_changed() {
        let n = parse_notification("%session-changed $1 my-session");
        assert!(matches!(n, Some(TmuxNotification::SessionChanged { .. })));
    }

    #[test]
    fn test_parse_window_add() {
        let n = parse_notification("%window-add @3");
        assert!(
            matches!(n, Some(TmuxNotification::WindowAdd { window_id }) if window_id == "3")
        );
    }

    #[test]
    fn test_parse_window_close() {
        let n = parse_notification("%window-close @2");
        assert!(
            matches!(n, Some(TmuxNotification::WindowClose { window_id }) if window_id == "2")
        );
    }

    #[test]
    fn test_parse_exit() {
        let n = parse_notification("%exit");
        assert!(matches!(n, Some(TmuxNotification::Exit)));
    }

    #[test]
    fn test_parse_layout_change() {
        let n = parse_notification("%layout-change @1 abc123,80x24,0,0");
        assert!(matches!(n, Some(TmuxNotification::LayoutChange { .. })));
    }

    #[test]
    fn test_controller_window_lifecycle() {
        let mut ctrl = TmuxController::new();
        ctrl.handle_notification(&TmuxNotification::WindowAdd {
            window_id: "1".to_string(),
        });
        assert_eq!(ctrl.windows.len(), 1);
        ctrl.handle_notification(&TmuxNotification::WindowRenamed {
            window_id: "1".to_string(),
            name: "dev".to_string(),
        });
        assert_eq!(ctrl.windows["1"].name, "dev");
        ctrl.handle_notification(&TmuxNotification::WindowClose {
            window_id: "1".to_string(),
        });
        assert_eq!(ctrl.windows.len(), 0);
    }

    #[test]
    fn test_list_sessions_command() {
        assert_eq!(
            TmuxController::connect_command(None),
            "tmux -CC new-session"
        );
        assert_eq!(
            TmuxController::connect_command(Some("dev")),
            "tmux -CC attach -t dev"
        );
    }
}
