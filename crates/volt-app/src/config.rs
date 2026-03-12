//! TOML configuration with hot-reload.
//!
//! Double-buffer pattern: parse new config on change, atomic swap into active config.
//! Covers: fonts, colors, keybindings, behavior, profiles.
//!
//! Config file: ~/.config/volt/config.toml

use std::path::PathBuf;

use serde::Deserialize;

/// Top-level Volt configuration.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct VoltConfig {
    pub font: FontConfig,
    pub window: WindowConfig,
    pub colors: ColorConfig,
    pub behavior: BehaviorConfig,
}

/// Font configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FontConfig {
    /// Font family name (e.g., "JetBrains Mono"). None = system monospace.
    pub family: Option<String>,
    /// Font size in points.
    pub size: f32,
    /// Line height multiplier (1.0 = tight, 1.2 = comfortable).
    pub line_height: f32,
}

/// Window configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    /// Initial window width in pixels.
    pub width: f64,
    /// Initial window height in pixels.
    pub height: f64,
    /// Window title (overridden by shell OSC sequences).
    pub title: String,
}

/// Color configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ColorConfig {
    /// Background color as [r, g, b] floats (0.0–1.0).
    pub background: [f32; 3],
}

/// Behavior configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BehaviorConfig {
    /// Shell to run. None = user's login shell.
    pub shell: Option<String>,
    /// Initial working directory.
    pub working_directory: Option<PathBuf>,
    /// Maximum scrollback lines.
    pub scrollback_lines: usize,
    /// Treat Option key as Meta (sends ESC prefix).
    pub option_as_meta: bool,
}

// VoltConfig derives Default from its field defaults.
// Each field type implements Default with custom values below.

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: None,
            size: 14.0,
            line_height: 1.2,
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 600.0,
            title: "Volt".into(),
        }
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            background: [0.1, 0.1, 0.12],
        }
    }
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            shell: None,
            working_directory: None,
            scrollback_lines: 10_000,
            option_as_meta: true,
        }
    }
}

/// Load configuration from `~/.config/volt/config.toml`.
/// Returns defaults if the file doesn't exist or can't be parsed.
pub fn load_config() -> VoltConfig {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => match toml::from_str(&contents) {
            Ok(config) => {
                tracing::info!("loaded config from {}", path.display());
                config
            }
            Err(e) => {
                tracing::warn!("failed to parse config: {e}, using defaults");
                VoltConfig::default()
            }
        },
        Err(_) => {
            tracing::info!("no config file at {}, using defaults", path.display());
            VoltConfig::default()
        }
    }
}

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home)
        .join(".config")
        .join("volt")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = VoltConfig::default();
        assert_eq!(config.font.size, 14.0);
        assert_eq!(config.window.width, 800.0);
        assert!(config.behavior.option_as_meta);
    }

    #[test]
    fn parse_partial_toml() {
        let toml = r#"
            [font]
            size = 16.0
            family = "Fira Code"
        "#;
        let config: VoltConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.font.size, 16.0);
        assert_eq!(config.font.family.as_deref(), Some("Fira Code"));
        // Other fields should be default
        assert_eq!(config.window.width, 800.0);
    }

    #[test]
    fn parse_full_toml() {
        let toml = r#"
            [font]
            family = "JetBrains Mono"
            size = 13.0
            line_height = 1.3

            [window]
            width = 1024.0
            height = 768.0
            title = "My Terminal"

            [colors]
            background = [0.0, 0.0, 0.0]

            [behavior]
            shell = "/bin/zsh"
            scrollback_lines = 50000
            option_as_meta = false
        "#;
        let config: VoltConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.font.family.as_deref(), Some("JetBrains Mono"));
        assert_eq!(config.colors.background, [0.0, 0.0, 0.0]);
        assert!(!config.behavior.option_as_meta);
    }
}
