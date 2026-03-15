//! AI Assistant — opens Claude Code in a split pane.
//! No API key needed — uses the `claude` CLI directly.

/// Check if claude CLI is available
pub fn is_claude_available() -> bool {
    std::process::Command::new("which")
        .arg("claude")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the path to the claude binary
pub fn claude_path() -> Option<String> {
    std::process::Command::new("which")
        .arg("claude")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Commands that can be used to start different AI modes
pub enum AiMode {
    /// Interactive chat session
    Chat,
    /// Ask a specific question
    Ask(String),
    /// Start in a specific directory context
    WithContext(String),
}

impl AiMode {
    /// Get the command and args to spawn
    pub fn command(&self) -> (String, Vec<String>) {
        let claude = claude_path().unwrap_or_else(|| "claude".to_string());
        match self {
            AiMode::Chat => (claude, vec![]),
            AiMode::Ask(question) => (claude, vec![question.clone()]),
            AiMode::WithContext(dir) => (claude, vec!["--cwd".to_string(), dir.clone()]),
        }
    }

    /// Get a display name for the tab
    pub fn tab_name(&self) -> &str {
        match self {
            AiMode::Chat => "AI",
            AiMode::Ask(_) => "AI Ask",
            AiMode::WithContext(_) => "AI",
        }
    }
}

/// Alternative AI tools that could be used if claude is not available
pub fn detect_available_ai() -> Vec<(&'static str, &'static str)> {
    let tools = vec![
        ("claude", "Claude Code"),
        ("aider", "Aider"),
        ("copilot", "GitHub Copilot CLI"),
        ("sgpt", "Shell GPT"),
    ];

    tools
        .into_iter()
        .filter(|(cmd, _)| {
            std::process::Command::new("which")
                .arg(cmd)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_mode_command() {
        let (cmd, args) = AiMode::Chat.command();
        assert!(cmd.contains("claude") || cmd == "claude");
        assert!(args.is_empty());
    }

    #[test]
    fn test_ai_mode_ask() {
        let (_, args) = AiMode::Ask("explain this error".to_string()).command();
        assert_eq!(args, vec!["explain this error"]);
    }

    #[test]
    fn test_tab_names() {
        assert_eq!(AiMode::Chat.tab_name(), "AI");
        assert_eq!(AiMode::Ask("test".into()).tab_name(), "AI Ask");
    }
}
