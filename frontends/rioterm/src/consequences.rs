use std::process::Command;

#[derive(Debug)]
pub struct ConsequencePreview {
    pub command: String,
    pub severity: Severity,
    pub description: String,
    pub details: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum Severity {
    Warning, // Potentially destructive
    Danger,  // Very destructive
    Info,    // Informational
}

/// List of safe commands that should never be intercepted
const SAFE_COMMANDS: &[&str] = &[
    "ls", "echo", "cat", "head", "tail", "grep", "find", "pwd", "cd", "env",
    "whoami", "date", "cal", "which", "where", "man", "help", "less", "more",
    "wc", "sort", "uniq", "diff", "file", "stat", "id", "uname", "hostname",
    "printenv", "true", "false", "test", "read",
];

/// Analyze a command and return a consequence preview if it's destructive.
/// Returns None for safe commands.
pub fn analyze_command(input: &str) -> Option<ConsequencePreview> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Parse first word as the command
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let cmd = parts.first()?;

    // Skip safe commands
    let base_cmd = cmd.rsplit('/').next().unwrap_or(cmd);
    if SAFE_COMMANDS.contains(&base_cmd) {
        return None;
    }

    // Skip if prefixed with ! (user override)
    if trimmed.starts_with('!') {
        return None;
    }

    // Check patterns
    check_rm(trimmed, &parts)
        .or_else(|| check_git_force_push(trimmed, &parts))
        .or_else(|| check_chmod_recursive(trimmed, &parts))
        .or_else(|| check_docker_prune(trimmed, &parts))
        .or_else(|| check_kubectl_delete(trimmed, &parts))
        .or_else(|| check_drop_table(trimmed))
}

fn check_rm(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    if parts.first()? != &"rm" {
        return None;
    }

    let has_recursive = parts
        .iter()
        .any(|p| p.contains('r') && p.starts_with('-'));
    let has_force = parts
        .iter()
        .any(|p| p.contains('f') && p.starts_with('-'));

    // Get file arguments (skip flags)
    let files: Vec<&str> = parts[1..]
        .iter()
        .filter(|p| !p.starts_with('-'))
        .copied()
        .collect();

    if files.is_empty() {
        return None;
    }

    let severity = if has_recursive && has_force {
        Severity::Danger
    } else if has_recursive {
        Severity::Warning
    } else {
        Severity::Warning
    };

    // Try to count affected files
    let mut details = Vec::new();
    for file in &files {
        if has_recursive {
            // Count files recursively
            if let Ok(output) = Command::new("find")
                .arg(file)
                .arg("-type")
                .arg("f")
                .output()
            {
                let count = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .count();
                if let Ok(du_output) = Command::new("du").arg("-sh").arg(file).output()
                {
                    let size = String::from_utf8_lossy(&du_output.stdout)
                        .split_whitespace()
                        .next()
                        .unwrap_or("?")
                        .to_string();
                    details.push(format!("{}: {} files, {}", file, count, size));
                } else {
                    details.push(format!("{}: {} files", file, count));
                }
            }
        } else {
            details.push(format!("Delete: {}", file));
        }
    }

    let desc = if has_recursive && has_force {
        "Force-removing files recursively (UNRECOVERABLE)".to_string()
    } else if has_recursive {
        "Removing files recursively".to_string()
    } else {
        format!("Removing {} file(s)", files.len())
    };

    Some(ConsequencePreview {
        command: input.to_string(),
        severity,
        description: desc,
        details,
    })
}

fn check_git_force_push(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    if parts.first()? != &"git" {
        return None;
    }
    if !parts.contains(&"push") {
        return None;
    }
    if !parts
        .iter()
        .any(|p| *p == "--force" || *p == "-f" || *p == "--force-with-lease")
    {
        return None;
    }

    Some(ConsequencePreview {
        command: input.to_string(),
        severity: Severity::Danger,
        description: "Force pushing will overwrite remote history".to_string(),
        details: vec!["Remote commits may be lost permanently".to_string()],
    })
}

fn check_chmod_recursive(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    if parts.first()? != &"chmod" {
        return None;
    }
    if !parts
        .iter()
        .any(|p| p.contains('R') && p.starts_with('-'))
    {
        return None;
    }

    Some(ConsequencePreview {
        command: input.to_string(),
        severity: Severity::Warning,
        description: "Recursively changing file permissions".to_string(),
        details: vec![],
    })
}

fn check_docker_prune(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    if parts.first()? != &"docker" {
        return None;
    }
    if !parts.contains(&"prune") && !parts.contains(&"rm") {
        return None;
    }

    Some(ConsequencePreview {
        command: input.to_string(),
        severity: Severity::Warning,
        description: "Docker cleanup — may remove containers/images/volumes".to_string(),
        details: vec![],
    })
}

fn check_kubectl_delete(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    if parts.first()? != &"kubectl" {
        return None;
    }
    if !parts.contains(&"delete") {
        return None;
    }

    Some(ConsequencePreview {
        command: input.to_string(),
        severity: Severity::Danger,
        description: "Deleting Kubernetes resources".to_string(),
        details: vec![],
    })
}

fn check_drop_table(input: &str) -> Option<ConsequencePreview> {
    let upper = input.to_uppercase();
    if upper.contains("DROP TABLE")
        || upper.contains("DROP DATABASE")
        || upper.contains("TRUNCATE")
    {
        Some(ConsequencePreview {
            command: input.to_string(),
            severity: Severity::Danger,
            description: "SQL destructive operation detected".to_string(),
            details: vec!["This will permanently delete data".to_string()],
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_commands_return_none() {
        assert!(analyze_command("ls -la").is_none());
        assert!(analyze_command("echo hello").is_none());
        assert!(analyze_command("cat foo.txt").is_none());
        assert!(analyze_command("grep pattern file").is_none());
    }

    #[test]
    fn test_empty_input_returns_none() {
        assert!(analyze_command("").is_none());
        assert!(analyze_command("   ").is_none());
    }

    #[test]
    fn test_override_prefix_returns_none() {
        assert!(analyze_command("!rm -rf /tmp/foo").is_none());
    }

    #[test]
    fn test_rm_detected() {
        let result = analyze_command("rm file.txt").unwrap();
        assert_eq!(result.severity, Severity::Warning);

        let result = analyze_command("rm -rf /tmp/foo").unwrap();
        assert_eq!(result.severity, Severity::Danger);

        let result = analyze_command("rm -r /tmp/foo").unwrap();
        assert_eq!(result.severity, Severity::Warning);
    }

    #[test]
    fn test_rm_no_files_returns_none() {
        assert!(analyze_command("rm -rf").is_none());
    }

    #[test]
    fn test_git_force_push_detected() {
        let result = analyze_command("git push --force origin main").unwrap();
        assert_eq!(result.severity, Severity::Danger);

        let result = analyze_command("git push -f").unwrap();
        assert_eq!(result.severity, Severity::Danger);

        let result =
            analyze_command("git push --force-with-lease origin main").unwrap();
        assert_eq!(result.severity, Severity::Danger);
    }

    #[test]
    fn test_git_normal_push_returns_none() {
        assert!(analyze_command("git push origin main").is_none());
    }

    #[test]
    fn test_chmod_recursive_detected() {
        let result = analyze_command("chmod -R 777 /tmp").unwrap();
        assert_eq!(result.severity, Severity::Warning);
    }

    #[test]
    fn test_docker_prune_detected() {
        let result = analyze_command("docker system prune").unwrap();
        assert_eq!(result.severity, Severity::Warning);
    }

    #[test]
    fn test_kubectl_delete_detected() {
        let result = analyze_command("kubectl delete pod my-pod").unwrap();
        assert_eq!(result.severity, Severity::Danger);
    }

    #[test]
    fn test_drop_table_detected() {
        let result = analyze_command("psql -c 'DROP TABLE users'").unwrap();
        assert_eq!(result.severity, Severity::Danger);
    }
}
