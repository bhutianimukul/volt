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
    "ls", "echo", "cat", "head", "tail", "grep", "find", "pwd", "cd", "env", "whoami",
    "date", "cal", "which", "where", "man", "help", "less", "more", "wc", "sort", "uniq",
    "diff", "file", "stat", "id", "uname", "hostname", "printenv", "true", "false",
    "test", "read",
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

    // Check for redirect overwrite before safe-command bail-out,
    // because `echo foo > file` is destructive despite `echo` being safe.
    if let Some(preview) = check_redirect_overwrite(trimmed) {
        return Some(preview);
    }

    // Skip safe commands
    let base_cmd = cmd.rsplit('/').next().unwrap_or(cmd);
    if SAFE_COMMANDS.contains(&base_cmd) {
        return None;
    }

    // Skip if prefixed with ! (user override)
    if trimmed.starts_with('!') {
        return None;
    }

    // Check patterns (20 destructive command categories)
    check_rm(trimmed, &parts)
        .or_else(|| check_git_force_push(trimmed, &parts))
        .or_else(|| check_chmod_recursive(trimmed, &parts))
        .or_else(|| check_docker_prune(trimmed, &parts))
        .or_else(|| check_kubectl_delete(trimmed, &parts))
        .or_else(|| check_drop_table(trimmed))
        .or_else(|| check_terraform_destroy(trimmed, &parts))
        .or_else(|| check_terraform_apply_no_plan(trimmed, &parts))
        .or_else(|| check_git_reset_hard(trimmed, &parts))
        .or_else(|| check_git_clean(trimmed, &parts))
        .or_else(|| check_dd(trimmed, &parts))
        .or_else(|| check_mkfs_format(trimmed, &parts))
        .or_else(|| check_kill_force(trimmed, &parts))
        .or_else(|| check_service_stop(trimmed, &parts))
        .or_else(|| check_pip_global(trimmed, &parts))
        .or_else(|| check_npm_global(trimmed, &parts))
        .or_else(|| check_sudo_rm(trimmed, &parts))
        .or_else(|| check_mv_dev_null(trimmed, &parts))
        .or_else(|| check_truncate_overwrite(trimmed, &parts))
        .or_else(|| check_chown_recursive(trimmed, &parts))
}

fn check_rm(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    if parts.first()? != &"rm" {
        return None;
    }

    let has_recursive = parts.iter().any(|p| p.contains('r') && p.starts_with('-'));
    let has_force = parts.iter().any(|p| p.contains('f') && p.starts_with('-'));

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

    let mut details = Vec::new();
    for file in &files {
        if has_recursive {
            details.push(format!("{}: recursive delete", file));
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
    if !parts.iter().any(|p| p.contains('R') && p.starts_with('-')) {
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
    // Only match SQL TRUNCATE TABLE, not the `truncate` CLI tool
    if upper.contains("DROP TABLE")
        || upper.contains("DROP DATABASE")
        || upper.contains("TRUNCATE TABLE")
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

// --- Pattern 7: terraform destroy ---
fn check_terraform_destroy(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    if parts.first()? != &"terraform" {
        return None;
    }
    if !parts.contains(&"destroy") {
        return None;
    }

    Some(ConsequencePreview {
        command: input.to_string(),
        severity: Severity::Danger,
        description: "Terraform destroy will tear down infrastructure resources".to_string(),
        details: vec!["All managed resources in the current state will be destroyed".to_string()],
    })
}

// --- Pattern 8: terraform apply without a plan file ---
fn check_terraform_apply_no_plan(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    if parts.first()? != &"terraform" {
        return None;
    }
    if !parts.contains(&"apply") {
        return None;
    }
    // If the user passed a .tfplan file, it's a reviewed plan — allow it
    if parts.iter().any(|p| p.ends_with(".tfplan")) {
        return None;
    }

    Some(ConsequencePreview {
        command: input.to_string(),
        severity: Severity::Warning,
        description: "Terraform apply without a saved plan — changes are unreviewed".to_string(),
        details: vec!["Run `terraform plan -out=plan.tfplan` first, then `terraform apply plan.tfplan`".to_string()],
    })
}

// --- Pattern 9: git reset --hard ---
fn check_git_reset_hard(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    if parts.first()? != &"git" {
        return None;
    }
    if !parts.contains(&"reset") {
        return None;
    }
    if !parts.contains(&"--hard") {
        return None;
    }

    Some(ConsequencePreview {
        command: input.to_string(),
        severity: Severity::Danger,
        description: "Hard reset will discard all uncommitted changes".to_string(),
        details: vec!["Staged and unstaged modifications will be lost permanently".to_string()],
    })
}

// --- Pattern 10: git clean -f ---
fn check_git_clean(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    if parts.first()? != &"git" {
        return None;
    }
    if !parts.contains(&"clean") {
        return None;
    }
    if !parts.iter().any(|p| p.starts_with('-') && p.contains('f')) {
        return None;
    }

    Some(ConsequencePreview {
        command: input.to_string(),
        severity: Severity::Warning,
        description: "Git clean will delete untracked files".to_string(),
        details: vec!["Untracked files not in .gitignore will be removed".to_string()],
    })
}

// --- Pattern 11: dd if= ---
fn check_dd(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    let base = parts.first()?.rsplit('/').next().unwrap_or(parts.first()?);
    if base != "dd" {
        return None;
    }
    if !parts.iter().any(|p| p.starts_with("if=")) {
        return None;
    }

    let of_target = parts
        .iter()
        .find(|p| p.starts_with("of="))
        .map(|p| p.trim_start_matches("of=").to_string());

    let mut details = vec!["dd performs raw block-level writes with no confirmation".to_string()];
    if let Some(target) = of_target {
        details.push(format!("Output target: {}", target));
    }

    Some(ConsequencePreview {
        command: input.to_string(),
        severity: Severity::Danger,
        description: "Disk-level write — may overwrite partitions or devices".to_string(),
        details,
    })
}

// --- Pattern 12: mkfs / format ---
fn check_mkfs_format(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    let base = parts.first()?.rsplit('/').next().unwrap_or(parts.first()?);
    if base.starts_with("mkfs") || base == "format" {
        Some(ConsequencePreview {
            command: input.to_string(),
            severity: Severity::Danger,
            description: "Formatting a disk will erase all data on the target".to_string(),
            details: vec!["All existing data on the device will be destroyed".to_string()],
        })
    } else {
        None
    }
}

// --- Pattern 13: kill -9 / killall ---
fn check_kill_force(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    let base = parts.first()?.rsplit('/').next().unwrap_or(parts.first()?);

    if base == "killall" {
        return Some(ConsequencePreview {
            command: input.to_string(),
            severity: Severity::Warning,
            description: "killall will terminate all processes matching the name".to_string(),
            details: vec!["Unsaved work in targeted processes will be lost".to_string()],
        });
    }

    if base == "kill" && parts.iter().any(|p| *p == "-9" || *p == "-KILL" || *p == "-SIGKILL") {
        return Some(ConsequencePreview {
            command: input.to_string(),
            severity: Severity::Warning,
            description: "SIGKILL forces immediate termination — process cannot clean up".to_string(),
            details: vec!["The process will not get a chance to save state or release resources".to_string()],
        });
    }

    None
}

// --- Pattern 14: systemctl stop / launchctl unload ---
fn check_service_stop(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    let base = parts.first()?.rsplit('/').next().unwrap_or(parts.first()?);

    if base == "systemctl" && parts.contains(&"stop") {
        let service = parts.last().unwrap_or(&"unknown");
        return Some(ConsequencePreview {
            command: input.to_string(),
            severity: Severity::Warning,
            description: format!("Stopping service: {}", service),
            details: vec!["Dependent services may also be affected".to_string()],
        });
    }

    if base == "launchctl" && (parts.contains(&"unload") || parts.contains(&"bootout")) {
        return Some(ConsequencePreview {
            command: input.to_string(),
            severity: Severity::Warning,
            description: "Unloading a launch daemon/agent".to_string(),
            details: vec!["The service will stop and not restart until re-loaded".to_string()],
        });
    }

    None
}

// --- Pattern 15: pip install without venv ---
fn check_pip_global(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    let base = parts.first()?.rsplit('/').next().unwrap_or(parts.first()?);
    if base != "pip" && base != "pip3" {
        return None;
    }
    if !parts.contains(&"install") {
        return None;
    }
    // If --user or inside a known venv indicator, skip
    if parts.contains(&"--user") {
        return None;
    }
    // Check for VIRTUAL_ENV — we can't read env here, so flag it as info
    Some(ConsequencePreview {
        command: input.to_string(),
        severity: Severity::Info,
        description: "pip install outside a virtual environment — may modify global packages".to_string(),
        details: vec!["Consider using a venv: `python -m venv .venv && source .venv/bin/activate`".to_string()],
    })
}

// --- Pattern 16: npm install -g ---
fn check_npm_global(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    let base = parts.first()?.rsplit('/').next().unwrap_or(parts.first()?);
    if base != "npm" {
        return None;
    }
    if !parts.contains(&"install") && !parts.contains(&"i") {
        return None;
    }
    if !parts.contains(&"-g") && !parts.contains(&"--global") {
        return None;
    }

    Some(ConsequencePreview {
        command: input.to_string(),
        severity: Severity::Info,
        description: "Global npm install — modifies system-wide node_modules".to_string(),
        details: vec!["Consider using npx or a project-local install instead".to_string()],
    })
}

// --- Pattern 17: sudo rm ---
fn check_sudo_rm(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    if parts.first()? != &"sudo" {
        return None;
    }
    // Find "rm" after "sudo"
    let after_sudo: Vec<&str> = parts[1..].to_vec();
    if after_sudo.first().copied() != Some("rm") {
        return None;
    }

    let has_recursive = after_sudo.iter().any(|p| p.starts_with('-') && p.contains('r'));
    let has_force = after_sudo.iter().any(|p| p.starts_with('-') && p.contains('f'));

    let severity = if has_recursive && has_force {
        Severity::Danger
    } else {
        Severity::Warning
    };

    Some(ConsequencePreview {
        command: input.to_string(),
        severity,
        description: "Elevated rm — deleting files as root".to_string(),
        details: vec!["Running rm with sudo bypasses normal permission safeguards".to_string()],
    })
}

// --- Pattern 18: mv to /dev/null ---
fn check_mv_dev_null(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    if parts.first()? != &"mv" {
        return None;
    }
    if !parts.contains(&"/dev/null") {
        return None;
    }

    Some(ConsequencePreview {
        command: input.to_string(),
        severity: Severity::Danger,
        description: "Moving files to /dev/null will destroy them".to_string(),
        details: vec!["Data moved to /dev/null is lost permanently".to_string()],
    })
}

// --- Pattern 19a: output-redirect overwrite (> file) ---
// Checked early in analyze_command, before safe-command bail-out.
fn check_redirect_overwrite(input: &str) -> Option<ConsequencePreview> {
    // Detect `> file` overwrite pattern (not `>>`)
    // This covers patterns like: `> important.log` or `echo > file`
    if input.contains(" > ") && !input.contains(" >> ") {
        // Only warn if the redirect target looks like a real file (not /dev/null)
        let after_redirect = input.split(" > ").last().unwrap_or("").trim();
        if !after_redirect.is_empty() && after_redirect != "/dev/null" {
            return Some(ConsequencePreview {
                command: input.to_string(),
                severity: Severity::Info,
                description: "Output redirect will overwrite file contents".to_string(),
                details: vec![format!("Target '{}' will be truncated before writing", after_redirect)],
            });
        }
    }
    None
}

// --- Pattern 19b: truncate command ---
fn check_truncate_overwrite(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    let base = parts.first()?.rsplit('/').next().unwrap_or(parts.first()?);

    if base == "truncate" {
        return Some(ConsequencePreview {
            command: input.to_string(),
            severity: Severity::Warning,
            description: "Truncate will erase file contents".to_string(),
            details: vec!["The file will be emptied or resized, losing existing data".to_string()],
        });
    }

    None
}

// --- Pattern 20: chown -R ---
fn check_chown_recursive(input: &str, parts: &[&str]) -> Option<ConsequencePreview> {
    if parts.first()? != &"chown" {
        return None;
    }
    if !parts.iter().any(|p| p.starts_with('-') && p.contains('R')) {
        return None;
    }

    Some(ConsequencePreview {
        command: input.to_string(),
        severity: Severity::Warning,
        description: "Recursively changing file ownership".to_string(),
        details: vec!["Incorrect ownership can break applications and services".to_string()],
    })
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

        let result = analyze_command("git push --force-with-lease origin main").unwrap();
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

    // --- Pattern 7: terraform destroy ---
    #[test]
    fn test_terraform_destroy_detected() {
        let result = analyze_command("terraform destroy").unwrap();
        assert_eq!(result.severity, Severity::Danger);

        let result = analyze_command("terraform destroy -auto-approve").unwrap();
        assert_eq!(result.severity, Severity::Danger);
    }

    // --- Pattern 8: terraform apply without plan ---
    #[test]
    fn test_terraform_apply_no_plan_detected() {
        let result = analyze_command("terraform apply").unwrap();
        assert_eq!(result.severity, Severity::Warning);

        let result = analyze_command("terraform apply -auto-approve").unwrap();
        assert_eq!(result.severity, Severity::Warning);
    }

    #[test]
    fn test_terraform_apply_with_plan_returns_none() {
        assert!(analyze_command("terraform apply plan.tfplan").is_none());
    }

    // --- Pattern 9: git reset --hard ---
    #[test]
    fn test_git_reset_hard_detected() {
        let result = analyze_command("git reset --hard").unwrap();
        assert_eq!(result.severity, Severity::Danger);

        let result = analyze_command("git reset --hard HEAD~3").unwrap();
        assert_eq!(result.severity, Severity::Danger);
    }

    #[test]
    fn test_git_reset_soft_returns_none() {
        assert!(analyze_command("git reset --soft HEAD~1").is_none());
    }

    // --- Pattern 10: git clean -f ---
    #[test]
    fn test_git_clean_detected() {
        let result = analyze_command("git clean -fd").unwrap();
        assert_eq!(result.severity, Severity::Warning);

        let result = analyze_command("git clean -f").unwrap();
        assert_eq!(result.severity, Severity::Warning);
    }

    #[test]
    fn test_git_clean_dry_run_returns_none() {
        assert!(analyze_command("git clean -n").is_none());
    }

    // --- Pattern 11: dd ---
    #[test]
    fn test_dd_detected() {
        let result = analyze_command("dd if=/dev/zero of=/dev/sda bs=1M").unwrap();
        assert_eq!(result.severity, Severity::Danger);
        assert!(result.details.iter().any(|d| d.contains("/dev/sda")));
    }

    #[test]
    fn test_dd_without_if_returns_none() {
        assert!(analyze_command("dd of=/dev/sda").is_none());
    }

    // --- Pattern 12: mkfs / format ---
    #[test]
    fn test_mkfs_detected() {
        let result = analyze_command("mkfs.ext4 /dev/sda1").unwrap();
        assert_eq!(result.severity, Severity::Danger);

        let result = analyze_command("mkfs -t xfs /dev/sdb").unwrap();
        assert_eq!(result.severity, Severity::Danger);
    }

    #[test]
    fn test_format_detected() {
        let result = analyze_command("format D:").unwrap();
        assert_eq!(result.severity, Severity::Danger);
    }

    // --- Pattern 13: kill -9 / killall ---
    #[test]
    fn test_kill_9_detected() {
        let result = analyze_command("kill -9 1234").unwrap();
        assert_eq!(result.severity, Severity::Warning);

        let result = analyze_command("kill -KILL 5678").unwrap();
        assert_eq!(result.severity, Severity::Warning);
    }

    #[test]
    fn test_killall_detected() {
        let result = analyze_command("killall firefox").unwrap();
        assert_eq!(result.severity, Severity::Warning);
    }

    #[test]
    fn test_kill_normal_returns_none() {
        assert!(analyze_command("kill 1234").is_none());
    }

    // --- Pattern 14: systemctl stop / launchctl unload ---
    #[test]
    fn test_systemctl_stop_detected() {
        let result = analyze_command("systemctl stop nginx").unwrap();
        assert_eq!(result.severity, Severity::Warning);
        assert!(result.description.contains("nginx"));
    }

    #[test]
    fn test_launchctl_unload_detected() {
        let result = analyze_command("launchctl unload com.example.daemon").unwrap();
        assert_eq!(result.severity, Severity::Warning);

        let result = analyze_command("launchctl bootout system/com.example.daemon").unwrap();
        assert_eq!(result.severity, Severity::Warning);
    }

    // --- Pattern 15: pip install without venv ---
    #[test]
    fn test_pip_global_detected() {
        let result = analyze_command("pip install requests").unwrap();
        assert_eq!(result.severity, Severity::Info);

        let result = analyze_command("pip3 install flask").unwrap();
        assert_eq!(result.severity, Severity::Info);
    }

    #[test]
    fn test_pip_user_returns_none() {
        assert!(analyze_command("pip install --user requests").is_none());
    }

    // --- Pattern 16: npm install -g ---
    #[test]
    fn test_npm_global_detected() {
        let result = analyze_command("npm install -g typescript").unwrap();
        assert_eq!(result.severity, Severity::Info);

        let result = analyze_command("npm i --global eslint").unwrap();
        assert_eq!(result.severity, Severity::Info);
    }

    #[test]
    fn test_npm_local_returns_none() {
        assert!(analyze_command("npm install lodash").is_none());
    }

    // --- Pattern 17: sudo rm ---
    #[test]
    fn test_sudo_rm_detected() {
        let result = analyze_command("sudo rm /etc/important.conf").unwrap();
        assert_eq!(result.severity, Severity::Warning);

        let result = analyze_command("sudo rm -rf /var/data").unwrap();
        assert_eq!(result.severity, Severity::Danger);
    }

    // --- Pattern 18: mv to /dev/null ---
    #[test]
    fn test_mv_dev_null_detected() {
        let result = analyze_command("mv important.log /dev/null").unwrap();
        assert_eq!(result.severity, Severity::Danger);
    }

    #[test]
    fn test_mv_normal_returns_none() {
        assert!(analyze_command("mv file.txt backup/").is_none());
    }

    // --- Pattern 19: truncate / redirect overwrite ---
    #[test]
    fn test_truncate_detected() {
        let result = analyze_command("truncate -s 0 /var/log/syslog").unwrap();
        assert_eq!(result.severity, Severity::Warning);
    }

    #[test]
    fn test_redirect_overwrite_detected() {
        let result = analyze_command("echo '' > important.conf").unwrap();
        assert_eq!(result.severity, Severity::Info);
    }

    #[test]
    fn test_redirect_append_returns_none() {
        assert!(analyze_command("echo 'line' >> log.txt").is_none());
    }

    // --- Pattern 20: chown -R ---
    #[test]
    fn test_chown_recursive_detected() {
        let result = analyze_command("chown -R root:root /var/www").unwrap();
        assert_eq!(result.severity, Severity::Warning);
    }

    #[test]
    fn test_chown_non_recursive_returns_none() {
        assert!(analyze_command("chown user:group file.txt").is_none());
    }
}
