//! Layout presets — save and restore pane arrangements.
//! Supports named presets and per-project .volt-layout.toml files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutPreset {
    pub name: String,
    pub description: Option<String>,
    pub panes: Vec<PaneSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneSpec {
    pub command: Option<String>,     // Command to run in pane
    pub working_dir: Option<String>, // Working directory
    pub split: SplitDirection,
    pub ratio: f32, // Size ratio (0.0 - 1.0)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SplitDirection {
    #[serde(rename = "right")]
    Right,
    #[serde(rename = "down")]
    Down,
    #[serde(rename = "root")]
    Root, // First pane (no split)
}

/// Built-in layout presets
pub fn builtin_presets() -> Vec<LayoutPreset> {
    vec![
        LayoutPreset {
            name: "dev".to_string(),
            description: Some(
                "Editor left, terminal right, logs bottom".to_string(),
            ),
            panes: vec![
                PaneSpec {
                    command: None,
                    working_dir: None,
                    split: SplitDirection::Root,
                    ratio: 0.6,
                },
                PaneSpec {
                    command: None,
                    working_dir: None,
                    split: SplitDirection::Right,
                    ratio: 0.4,
                },
                PaneSpec {
                    command: None,
                    working_dir: None,
                    split: SplitDirection::Down,
                    ratio: 0.3,
                },
            ],
        },
        LayoutPreset {
            name: "side-by-side".to_string(),
            description: Some("Two equal panes side by side".to_string()),
            panes: vec![
                PaneSpec {
                    command: None,
                    working_dir: None,
                    split: SplitDirection::Root,
                    ratio: 0.5,
                },
                PaneSpec {
                    command: None,
                    working_dir: None,
                    split: SplitDirection::Right,
                    ratio: 0.5,
                },
            ],
        },
        LayoutPreset {
            name: "top-bottom".to_string(),
            description: Some("Two equal panes stacked".to_string()),
            panes: vec![
                PaneSpec {
                    command: None,
                    working_dir: None,
                    split: SplitDirection::Root,
                    ratio: 0.5,
                },
                PaneSpec {
                    command: None,
                    working_dir: None,
                    split: SplitDirection::Down,
                    ratio: 0.5,
                },
            ],
        },
        LayoutPreset {
            name: "quad".to_string(),
            description: Some("Four equal panes in a grid".to_string()),
            panes: vec![
                PaneSpec {
                    command: None,
                    working_dir: None,
                    split: SplitDirection::Root,
                    ratio: 0.5,
                },
                PaneSpec {
                    command: None,
                    working_dir: None,
                    split: SplitDirection::Right,
                    ratio: 0.5,
                },
                PaneSpec {
                    command: None,
                    working_dir: None,
                    split: SplitDirection::Down,
                    ratio: 0.5,
                },
                PaneSpec {
                    command: None,
                    working_dir: None,
                    split: SplitDirection::Down,
                    ratio: 0.5,
                },
            ],
        },
        LayoutPreset {
            name: "monitoring".to_string(),
            description: Some(
                "Three panes: main + two stacked right".to_string(),
            ),
            panes: vec![
                PaneSpec {
                    command: None,
                    working_dir: None,
                    split: SplitDirection::Root,
                    ratio: 0.65,
                },
                PaneSpec {
                    command: Some("htop".to_string()),
                    working_dir: None,
                    split: SplitDirection::Right,
                    ratio: 0.35,
                },
                PaneSpec {
                    command: Some(
                        "tail -f /var/log/system.log".to_string(),
                    ),
                    working_dir: None,
                    split: SplitDirection::Down,
                    ratio: 0.5,
                },
            ],
        },
    ]
}

/// Saved user presets stored in ~/.config/volt/layouts/
#[derive(Debug)]
pub struct LayoutManager {
    presets: HashMap<String, LayoutPreset>,
    layouts_dir: PathBuf,
}

impl LayoutManager {
    pub fn new() -> Self {
        let layouts_dir =
            rio_backend::config::config_dir_path().join("layouts");
        let _ = std::fs::create_dir_all(&layouts_dir);

        let mut presets = HashMap::new();
        // Load built-in presets
        for preset in builtin_presets() {
            presets.insert(preset.name.clone(), preset);
        }
        // Load user presets from disk
        if let Ok(entries) = std::fs::read_dir(&layouts_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "toml") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(preset) =
                            toml::from_str::<LayoutPreset>(&content)
                        {
                            presets.insert(preset.name.clone(), preset);
                        }
                    }
                }
            }
        }

        Self {
            presets,
            layouts_dir,
        }
    }

    /// Get a preset by name
    pub fn get(&self, name: &str) -> Option<&LayoutPreset> {
        self.presets.get(name)
    }

    /// Save a layout preset to disk
    pub fn save(&mut self, preset: LayoutPreset) -> Result<(), String> {
        let path = self.layouts_dir.join(format!("{}.toml", preset.name));
        let content = toml::to_string_pretty(&preset)
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write: {}", e))?;
        self.presets.insert(preset.name.clone(), preset);
        Ok(())
    }

    /// List all available presets
    pub fn list(&self) -> Vec<&LayoutPreset> {
        let mut presets: Vec<&LayoutPreset> = self.presets.values().collect();
        presets.sort_by_key(|p| &p.name);
        presets
    }

    /// Delete a user preset (cannot delete built-in)
    pub fn delete(&mut self, name: &str) -> bool {
        let path = self.layouts_dir.join(format!("{}.toml", name));
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
        self.presets.remove(name).is_some()
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Load a project-specific layout from .volt-layout.toml in the given directory
pub fn load_project_layout(dir: &Path) -> Option<LayoutPreset> {
    let layout_file = dir.join(".volt-layout.toml");
    if !layout_file.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&layout_file).ok()?;
    toml::from_str(&content).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_presets() {
        let presets = builtin_presets();
        assert!(presets.len() >= 5);
        assert!(presets.iter().any(|p| p.name == "dev"));
        assert!(presets.iter().any(|p| p.name == "quad"));
    }

    #[test]
    fn test_layout_manager() {
        let mgr = LayoutManager::new();
        assert!(mgr.get("dev").is_some());
        assert!(mgr.get("nonexistent").is_none());
        assert!(mgr.list().len() >= 5);
    }

    #[test]
    fn test_preset_serialization() {
        let preset = LayoutPreset {
            name: "test".to_string(),
            description: Some("Test layout".to_string()),
            panes: vec![
                PaneSpec {
                    command: None,
                    working_dir: None,
                    split: SplitDirection::Root,
                    ratio: 0.5,
                },
                PaneSpec {
                    command: Some("htop".to_string()),
                    working_dir: None,
                    split: SplitDirection::Right,
                    ratio: 0.5,
                },
            ],
        };
        let toml_str = toml::to_string_pretty(&preset).unwrap();
        assert!(toml_str.contains("test"));
        assert!(toml_str.contains("htop"));

        // Roundtrip
        let parsed: LayoutPreset = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.name, "test");
        assert_eq!(parsed.panes.len(), 2);
    }
}
