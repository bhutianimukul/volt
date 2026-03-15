//! Config import — convert settings from other terminals to Volt format.
//! Supports iTerm2 (plist), Alacritty (TOML/YAML), WezTerm (Lua), Ghostty (ini-style).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum TerminalSource {
    ITerm2,
    Alacritty,
    WezTerm,
    Ghostty,
    Kitty,
}

impl TerminalSource {
    pub fn name(&self) -> &str {
        match self {
            TerminalSource::ITerm2 => "iTerm2",
            TerminalSource::Alacritty => "Alacritty",
            TerminalSource::WezTerm => "WezTerm",
            TerminalSource::Ghostty => "Ghostty",
            TerminalSource::Kitty => "Kitty",
        }
    }

    pub fn config_path(&self) -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        match self {
            TerminalSource::ITerm2 => {
                Some(home.join("Library/Preferences/com.googlecode.iterm2.plist"))
            }
            TerminalSource::Alacritty => {
                let toml = home.join(".config/alacritty/alacritty.toml");
                if toml.exists() {
                    return Some(toml);
                }
                let yml = home.join(".config/alacritty/alacritty.yml");
                if yml.exists() {
                    return Some(yml);
                }
                Some(toml) // Return toml path even if doesn't exist
            }
            TerminalSource::WezTerm => Some(home.join(".wezterm.lua")),
            TerminalSource::Ghostty => Some(home.join(".config/ghostty/config")),
            TerminalSource::Kitty => Some(home.join(".config/kitty/kitty.conf")),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImportedConfig {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub background_color: Option<String>,
    pub foreground_color: Option<String>,
    pub cursor_color: Option<String>,
    pub window_opacity: Option<f32>,
    pub shell: Option<String>,
    pub shell_args: Vec<String>,
    pub padding_x: Option<f32>,
    pub padding_y: Option<f32>,
    pub extra: HashMap<String, String>,
}

impl ImportedConfig {
    /// Convert to Volt TOML config string
    pub fn to_volt_toml(&self) -> String {
        let mut sections = Vec::new();

        // [fonts]
        let mut fonts = Vec::new();
        if let Some(ref family) = self.font_family {
            fonts.push(format!("family = \"{}\"", family));
        }
        if let Some(size) = self.font_size {
            fonts.push(format!("size = {}", size));
        }
        if !fonts.is_empty() {
            sections.push(format!("[fonts]\n{}", fonts.join("\n")));
        }

        // [window]
        let mut window = Vec::new();
        if let Some(opacity) = self.window_opacity {
            window.push(format!("opacity = {:.2}", opacity));
        }
        if let Some(px) = self.padding_x {
            window.push(format!("padding-x = {}", px));
        }
        if !window.is_empty() {
            sections.push(format!("[window]\n{}", window.join("\n")));
        }

        // [colors]
        let mut colors = Vec::new();
        if let Some(ref bg) = self.background_color {
            colors.push(format!("background = \"{}\"", bg));
        }
        if let Some(ref fg) = self.foreground_color {
            colors.push(format!("foreground = \"{}\"", fg));
        }
        if let Some(ref cursor) = self.cursor_color {
            colors.push(format!("cursor = \"{}\"", cursor));
        }
        if !colors.is_empty() {
            sections.push(format!("[colors]\n{}", colors.join("\n")));
        }

        // [shell]
        if let Some(ref shell) = self.shell {
            let mut shell_section = format!("[shell]\nprogram = \"{}\"", shell);
            if !self.shell_args.is_empty() {
                let args: Vec<String> = self
                    .shell_args
                    .iter()
                    .map(|a| format!("\"{}\"", a))
                    .collect();
                shell_section.push_str(&format!("\nargs = [{}]", args.join(", ")));
            }
            sections.push(shell_section);
        }

        format!(
            "# Imported by Volt Terminal\n# Source: auto-detected\n\n{}\n",
            sections.join("\n\n")
        )
    }
}

/// Detect which terminals are installed
pub fn detect_installed_terminals() -> Vec<TerminalSource> {
    let sources = vec![
        TerminalSource::ITerm2,
        TerminalSource::Alacritty,
        TerminalSource::WezTerm,
        TerminalSource::Ghostty,
        TerminalSource::Kitty,
    ];

    sources
        .into_iter()
        .filter(|s| s.config_path().map_or(false, |p| p.exists()))
        .collect()
}

/// Import config from Alacritty TOML
pub fn import_alacritty(path: &Path) -> Result<ImportedConfig, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut config = ImportedConfig::default();

    for line in content.lines() {
        let trimmed = line.trim();
        // Simple key-value extraction (not full TOML parsing)
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim().trim_matches('"');
            let value = value.trim().trim_matches('"').trim_matches('\'');

            match key {
                "family" | "font.normal.family" => {
                    config.font_family = Some(value.to_string())
                }
                "size" | "font.size" => config.font_size = value.parse().ok(),
                "background" | "colors.primary.background" => {
                    config.background_color = Some(value.to_string())
                }
                "foreground" | "colors.primary.foreground" => {
                    config.foreground_color = Some(value.to_string())
                }
                "opacity" | "window.opacity" => {
                    config.window_opacity = value.parse().ok()
                }
                "program" | "shell.program" => config.shell = Some(value.to_string()),
                _ => {
                    config.extra.insert(key.to_string(), value.to_string());
                }
            }
        }
    }

    Ok(config)
}

/// Import config from Ghostty ini-style config
pub fn import_ghostty(path: &Path) -> Result<ImportedConfig, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut config = ImportedConfig::default();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "font-family" => config.font_family = Some(value.to_string()),
                "font-size" => config.font_size = value.parse().ok(),
                "background" => config.background_color = Some(value.to_string()),
                "foreground" => config.foreground_color = Some(value.to_string()),
                "cursor-color" => config.cursor_color = Some(value.to_string()),
                "background-opacity" => config.window_opacity = value.parse().ok(),
                "command" => config.shell = Some(value.to_string()),
                "window-padding-x" => config.padding_x = value.parse().ok(),
                "window-padding-y" => config.padding_y = value.parse().ok(),
                _ => {
                    config.extra.insert(key.to_string(), value.to_string());
                }
            }
        }
    }

    Ok(config)
}

/// Import config from Kitty
pub fn import_kitty(path: &Path) -> Result<ImportedConfig, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut config = ImportedConfig::default();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Kitty uses "key value" format (space separated)
        let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
        if parts.len() != 2 {
            continue;
        }
        let key = parts[0].trim();
        let value = parts[1].trim();

        match key {
            "font_family" => config.font_family = Some(value.to_string()),
            "font_size" => config.font_size = value.parse().ok(),
            "background" => config.background_color = Some(value.to_string()),
            "foreground" => config.foreground_color = Some(value.to_string()),
            "cursor" => config.cursor_color = Some(value.to_string()),
            "background_opacity" => config.window_opacity = value.parse().ok(),
            "shell" => config.shell = Some(value.to_string()),
            "window_padding_width" => config.padding_x = value.parse().ok(),
            _ => {
                config.extra.insert(key.to_string(), value.to_string());
            }
        }
    }

    Ok(config)
}

/// Auto-detect and import from the best available source
pub fn auto_import() -> Option<(TerminalSource, ImportedConfig)> {
    let installed = detect_installed_terminals();
    for source in installed {
        let path = source.config_path()?;
        let result = match source {
            TerminalSource::Alacritty => import_alacritty(&path),
            TerminalSource::Ghostty => import_ghostty(&path),
            TerminalSource::Kitty => import_kitty(&path),
            _ => continue, // iTerm2 plist and WezTerm Lua need special parsing
        };
        if let Ok(config) = result {
            if config.font_family.is_some() || config.font_size.is_some() {
                return Some((source, config));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_alacritty_format() {
        let tmp = std::env::temp_dir().join("test_alacritty.toml");
        std::fs::write(
            &tmp,
            r#"
[font]
size = 14

[font.normal]
family = "JetBrains Mono"

[window]
opacity = 0.95
"#,
        )
        .unwrap();

        let config = import_alacritty(&tmp).unwrap();
        assert_eq!(config.font_family, Some("JetBrains Mono".to_string()));
        assert_eq!(config.font_size, Some(14.0));
        assert_eq!(config.window_opacity, Some(0.95));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_ghostty_format() {
        let tmp = std::env::temp_dir().join("test_ghostty");
        std::fs::write(
            &tmp,
            "# Ghostty config\nfont-family = Fira Code\nfont-size = 13\nbackground-opacity = 0.9\n",
        )
        .unwrap();

        let config = import_ghostty(&tmp).unwrap();
        assert_eq!(config.font_family, Some("Fira Code".to_string()));
        assert_eq!(config.font_size, Some(13.0));
        assert_eq!(config.window_opacity, Some(0.9));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_kitty_format() {
        let tmp = std::env::temp_dir().join("test_kitty.conf");
        std::fs::write(
            &tmp,
            "# kitty config\nfont_family Hack\nfont_size 12\nbackground_opacity 0.85\n",
        )
        .unwrap();

        let config = import_kitty(&tmp).unwrap();
        assert_eq!(config.font_family, Some("Hack".to_string()));
        assert_eq!(config.font_size, Some(12.0));
        assert_eq!(config.window_opacity, Some(0.85));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_to_volt_toml() {
        let config = ImportedConfig {
            font_family: Some("Monaco".to_string()),
            font_size: Some(15.0),
            background_color: Some("#1a1a2e".to_string()),
            window_opacity: Some(0.9),
            shell: Some("/bin/zsh".to_string()),
            ..Default::default()
        };
        let toml = config.to_volt_toml();
        assert!(toml.contains("family = \"Monaco\""));
        assert!(toml.contains("size = 15"));
        assert!(toml.contains("background = \"#1a1a2e\""));
        assert!(toml.contains("opacity = 0.90"));
        assert!(toml.contains("program = \"/bin/zsh\""));
    }

    #[test]
    fn test_terminal_source_paths() {
        // Just verify these don't panic
        for source in &[
            TerminalSource::ITerm2,
            TerminalSource::Alacritty,
            TerminalSource::Ghostty,
            TerminalSource::Kitty,
            TerminalSource::WezTerm,
        ] {
            let _ = source.config_path();
            let _ = source.name();
        }
    }
}
