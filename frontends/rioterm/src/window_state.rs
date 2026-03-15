//! Window state restoration — save/restore window layout across restarts.
//! Persists to ~/.config/volt/window-state.json

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub windows: Vec<SavedWindow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedWindow {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub is_fullscreen: bool,
    pub tabs: Vec<SavedTab>,
    pub active_tab: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedTab {
    pub title: Option<String>,
    pub working_dir: Option<String>,
    pub shell: Option<String>,
}

impl WindowState {
    pub fn new() -> Self {
        Self { windows: Vec::new() }
    }

    /// Save a window's state
    pub fn add_window(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        is_fullscreen: bool,
    ) -> usize {
        let idx = self.windows.len();
        self.windows.push(SavedWindow {
            x,
            y,
            width,
            height,
            is_fullscreen,
            tabs: Vec::new(),
            active_tab: 0,
        });
        idx
    }

    /// Add a tab to a saved window
    pub fn add_tab(
        &mut self,
        window_idx: usize,
        title: Option<String>,
        working_dir: Option<String>,
    ) {
        if let Some(window) = self.windows.get_mut(window_idx) {
            window.tabs.push(SavedTab {
                title,
                working_dir,
                shell: None,
            });
        }
    }

    /// Save state to disk
    pub fn save(&self) -> Result<(), String> {
        let path = state_file_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json =
            serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
    }

    /// Load state from disk
    pub fn load() -> Self {
        let path = state_file_path();
        if !path.exists() {
            return Self::new();
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                serde_json::from_str(&content).unwrap_or_else(|_| Self::new())
            }
            Err(_) => Self::new(),
        }
    }

    /// Clear saved state
    pub fn clear() {
        let path = state_file_path();
        let _ = std::fs::remove_file(&path);
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn total_tabs(&self) -> usize {
        self.windows.iter().map(|w| w.tabs.len()).sum()
    }
}

impl Default for WindowState {
    fn default() -> Self {
        Self::new()
    }
}

fn state_file_path() -> PathBuf {
    rio_backend::config::config_dir_path().join("window-state.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_window() {
        let mut state = WindowState::new();
        let idx = state.add_window(100.0, 200.0, 800.0, 600.0, false);
        assert_eq!(idx, 0);
        assert_eq!(state.window_count(), 1);
        assert_eq!(state.windows[0].width, 800.0);
    }

    #[test]
    fn test_add_tabs() {
        let mut state = WindowState::new();
        let idx = state.add_window(0.0, 0.0, 800.0, 600.0, false);
        state.add_tab(idx, Some("main".into()), Some("/home".into()));
        state.add_tab(idx, Some("server".into()), Some("/var/www".into()));
        assert_eq!(state.windows[0].tabs.len(), 2);
        assert_eq!(state.total_tabs(), 2);
    }

    #[test]
    fn test_serialization() {
        let mut state = WindowState::new();
        let idx = state.add_window(100.0, 200.0, 1024.0, 768.0, true);
        state.add_tab(idx, Some("dev".into()), Some("/projects".into()));

        let json = serde_json::to_string(&state).unwrap();
        let restored: WindowState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.window_count(), 1);
        assert!(restored.windows[0].is_fullscreen);
        assert_eq!(restored.windows[0].tabs[0].title, Some("dev".into()));
    }

    #[test]
    fn test_load_does_not_crash() {
        // Load from disk — may have saved state from previous runs
        let state = WindowState::load();
        // Just verify it doesn't panic and returns valid state
        let _ = state.window_count();
        let _ = state.total_tabs();
    }
}
