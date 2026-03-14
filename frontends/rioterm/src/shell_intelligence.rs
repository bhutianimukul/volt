use std::path::Path;

/// Detected project type based on files in the directory
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectType {
    Rust,      // Cargo.toml
    Node,      // package.json
    Python,    // pyproject.toml, setup.py, requirements.txt
    Go,        // go.mod
    Ruby,      // Gemfile
    Java,      // pom.xml, build.gradle
    Docker,    // Dockerfile, docker-compose.yml
    Terraform, // *.tf files
    Unknown,
}

impl ProjectType {
    pub fn name(&self) -> &str {
        match self {
            ProjectType::Rust => "Rust",
            ProjectType::Node => "Node.js",
            ProjectType::Python => "Python",
            ProjectType::Go => "Go",
            ProjectType::Ruby => "Ruby",
            ProjectType::Java => "Java",
            ProjectType::Docker => "Docker",
            ProjectType::Terraform => "Terraform",
            ProjectType::Unknown => "Unknown",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            ProjectType::Rust => "\u{1F980}",
            ProjectType::Node => "\u{2B22}",
            ProjectType::Python => "\u{1F40D}",
            ProjectType::Go => "\u{1F439}",
            ProjectType::Ruby => "\u{1F48E}",
            ProjectType::Java => "\u{2615}",
            ProjectType::Docker => "\u{1F433}",
            ProjectType::Terraform => "\u{1F3D7}",
            ProjectType::Unknown => "\u{1F4C1}",
        }
    }

    /// Suggest common commands for this project type
    pub fn suggested_commands(&self) -> Vec<&str> {
        match self {
            ProjectType::Rust => vec![
                "cargo build",
                "cargo test",
                "cargo run",
                "cargo clippy",
                "cargo fmt",
            ],
            ProjectType::Node => {
                vec!["npm install", "npm test", "npm run dev", "npm run build"]
            }
            ProjectType::Python => vec![
                "pip install -r requirements.txt",
                "pytest",
                "python -m venv .venv",
            ],
            ProjectType::Go => vec!["go build", "go test ./...", "go run ."],
            ProjectType::Ruby => vec!["bundle install", "bundle exec rspec", "rails s"],
            ProjectType::Java => vec!["mvn clean install", "gradle build"],
            ProjectType::Docker => {
                vec!["docker build .", "docker-compose up", "docker ps"]
            }
            ProjectType::Terraform => {
                vec!["terraform plan", "terraform apply", "terraform init"]
            }
            ProjectType::Unknown => vec![],
        }
    }
}

/// Detect project type by examining files in the directory
pub fn detect_project(dir: &Path) -> ProjectType {
    if dir.join("Cargo.toml").exists() {
        return ProjectType::Rust;
    }
    if dir.join("package.json").exists() {
        return ProjectType::Node;
    }
    if dir.join("go.mod").exists() {
        return ProjectType::Go;
    }
    if dir.join("Gemfile").exists() {
        return ProjectType::Ruby;
    }
    if dir.join("pyproject.toml").exists()
        || dir.join("setup.py").exists()
        || dir.join("requirements.txt").exists()
    {
        return ProjectType::Python;
    }
    if dir.join("pom.xml").exists() || dir.join("build.gradle").exists() {
        return ProjectType::Java;
    }
    if dir.join("Dockerfile").exists() || dir.join("docker-compose.yml").exists() {
        return ProjectType::Docker;
    }
    // Check for .tf files
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().extension().map_or(false, |e| e == "tf") {
                return ProjectType::Terraform;
            }
        }
    }
    ProjectType::Unknown
}

/// Detect git branch in the given directory
pub fn detect_git_branch(dir: &Path) -> Option<String> {
    let head_file = dir.join(".git/HEAD");
    if !head_file.exists() {
        // Try parent directories
        let mut current = dir.to_path_buf();
        loop {
            let git_dir = current.join(".git/HEAD");
            if git_dir.exists() {
                return parse_git_head(&git_dir);
            }
            if !current.pop() {
                break;
            }
        }
        return None;
    }
    parse_git_head(&head_file)
}

fn parse_git_head(head_file: &Path) -> Option<String> {
    let content = std::fs::read_to_string(head_file).ok()?;
    let trimmed = content.trim();
    if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
        Some(branch.to_string())
    } else {
        // Detached HEAD - show short hash
        Some(trimmed.chars().take(7).collect())
    }
}

/// Known patterns for secrets/API keys
const SECRET_PATTERNS: &[(&str, &str)] = &[
    ("AKIA", "AWS Access Key"),
    ("ghp_", "GitHub Personal Token"),
    ("gho_", "GitHub OAuth Token"),
    ("sk-live-", "Stripe Live Key"),
    ("sk-test-", "Stripe Test Key"),
    ("sk-", "OpenAI/Anthropic API Key"),
    ("xoxb-", "Slack Bot Token"),
    ("xoxp-", "Slack User Token"),
    ("SG.", "SendGrid API Key"),
    ("-----BEGIN RSA PRIVATE KEY-----", "RSA Private Key"),
    ("-----BEGIN PRIVATE KEY-----", "Private Key"),
];

/// Scan a command for potential secrets
pub fn detect_secrets(command: &str) -> Vec<(&'static str, &'static str)> {
    let mut found = Vec::new();
    for (pattern, name) in SECRET_PATTERNS {
        if command.contains(pattern) {
            found.push((*pattern, *name));
        }
    }
    found
}

/// Format a duration in milliseconds to a human-readable string
pub fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else if ms < 3_600_000 {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        format!("{}m{}s", mins, secs)
    } else {
        let hours = ms / 3_600_000;
        let mins = (ms % 3_600_000) / 60_000;
        format!("{}h{}m", hours, mins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_detection() {
        let secrets = detect_secrets("curl -H 'Authorization: Bearer sk-live-abc123'");
        assert!(!secrets.is_empty());
        assert!(secrets.iter().any(|(_, name)| *name == "Stripe Live Key"));
    }

    #[test]
    fn test_no_false_positive() {
        let secrets = detect_secrets("echo hello world");
        assert!(secrets.is_empty());
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(500), "500ms");
        assert_eq!(format_duration(5000), "5.0s");
        assert_eq!(format_duration(65000), "1m5s");
        assert_eq!(format_duration(3_665_000), "1h1m");
    }

    #[test]
    fn test_detect_project_rust() {
        let dir = std::env::temp_dir().join("volt_test_rust");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("Cargo.toml"), "").unwrap();
        assert_eq!(detect_project(&dir), ProjectType::Rust);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_detect_project_node() {
        let dir = std::env::temp_dir().join("volt_test_node");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("package.json"), "{}").unwrap();
        assert_eq!(detect_project(&dir), ProjectType::Node);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_detect_project_unknown() {
        let dir = std::env::temp_dir().join("volt_test_unknown");
        let _ = std::fs::create_dir_all(&dir);
        assert_eq!(detect_project(&dir), ProjectType::Unknown);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_project_type_suggestions() {
        assert!(!ProjectType::Rust.suggested_commands().is_empty());
        assert!(ProjectType::Unknown.suggested_commands().is_empty());
    }

    #[test]
    fn test_project_type_name_and_icon() {
        assert_eq!(ProjectType::Rust.name(), "Rust");
        assert!(!ProjectType::Rust.icon().is_empty());
    }
}
