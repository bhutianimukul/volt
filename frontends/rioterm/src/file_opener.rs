//! Clickable file paths — Cmd+Click on file:line opens in editor.

use std::path::Path;
use std::process::Command;

/// Try to parse a file:line:col reference from text
pub fn parse_file_reference(text: &str) -> Option<(String, Option<u32>, Option<u32>)> {
    let trimmed = text.trim();

    // Patterns: file:line:col, file:line, file
    let parts: Vec<&str> = trimmed.splitn(3, ':').collect();

    let file = parts[0].trim();
    if file.is_empty() {
        return None;
    }

    // Must look like a file path
    if !Path::new(file).exists() && !file.contains('/') && !file.contains('.') {
        return None;
    }

    let line = parts.get(1).and_then(|s| s.trim().parse::<u32>().ok());
    let col = parts.get(2).and_then(|s| s.trim().parse::<u32>().ok());

    Some((file.to_string(), line, col))
}

/// Open a file in the configured editor at the given line
pub fn open_in_editor(file: &str, line: Option<u32>, col: Option<u32>) {
    let editor =
        std::env::var("EDITOR").unwrap_or_else(|_| "code".to_string());
    let editor_base = editor.split_whitespace().next().unwrap_or("code");

    let result = match editor_base {
        "code" | "code-insiders" => {
            // VS Code: code --goto file:line:col
            let goto = match (line, col) {
                (Some(l), Some(c)) => format!("{}:{}:{}", file, l, c),
                (Some(l), None) => format!("{}:{}", file, l),
                _ => file.to_string(),
            };
            Command::new(&editor).arg("--goto").arg(&goto).spawn()
        }
        "nvim" | "vim" | "vi" => match line {
            Some(l) => Command::new(&editor)
                .arg(format!("+{}", l))
                .arg(file)
                .spawn(),
            None => Command::new(&editor).arg(file).spawn(),
        },
        "subl" | "sublime" => {
            let target = match line {
                Some(l) => format!("{}:{}", file, l),
                None => file.to_string(),
            };
            Command::new(&editor).arg(&target).spawn()
        }
        _ => Command::new(&editor).arg(file).spawn(),
    };

    match result {
        Ok(_) => tracing::info!("Opened {} in {}", file, editor),
        Err(e) => tracing::warn!("Failed to open {} in {}: {}", file, editor, e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_file_line_col() {
        let (f, l, c) = parse_file_reference("src/main.rs:42:5").unwrap();
        assert_eq!(f, "src/main.rs");
        assert_eq!(l, Some(42));
        assert_eq!(c, Some(5));
    }

    #[test]
    fn test_parse_file_line() {
        let (f, l, c) = parse_file_reference("Cargo.toml:10").unwrap();
        assert_eq!(f, "Cargo.toml");
        assert_eq!(l, Some(10));
        assert_eq!(c, None);
    }

    #[test]
    fn test_parse_no_match() {
        assert!(parse_file_reference("hello world").is_none());
    }
}
