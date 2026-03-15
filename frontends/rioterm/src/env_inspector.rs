//! Environment variable inspector — searchable, categorized view of env vars.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum EnvCategory {
    Shell,
    Path,
    Language,
    Terminal,
    Editor,
    Git,
    Cloud,
    Custom,
    Other,
}

impl EnvCategory {
    pub fn name(&self) -> &str {
        match self {
            EnvCategory::Shell => "Shell",
            EnvCategory::Path => "Paths",
            EnvCategory::Language => "Language/Runtime",
            EnvCategory::Terminal => "Terminal",
            EnvCategory::Editor => "Editor",
            EnvCategory::Git => "Git",
            EnvCategory::Cloud => "Cloud/DevOps",
            EnvCategory::Custom => "Custom",
            EnvCategory::Other => "Other",
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
    pub category: EnvCategory,
    pub is_secret: bool,
}

/// Categorize an environment variable by its name
pub fn categorize(key: &str) -> EnvCategory {
    let k = key.to_uppercase();

    // Shell
    if matches!(k.as_str(), "SHELL" | "BASH" | "ZSH_VERSION" | "FISH_VERSION"
        | "SHLVL" | "TERM" | "TERM_PROGRAM" | "TERM_PROGRAM_VERSION"
        | "COLORTERM" | "LANG" | "LC_ALL" | "LC_CTYPE" | "USER" | "LOGNAME"
        | "HOME" | "TMPDIR" | "OLDPWD" | "PWD" | "HISTFILE" | "HISTSIZE") {
        return EnvCategory::Shell;
    }

    // Paths
    if k.contains("PATH") || k.contains("DIR") || k == "CDPATH" || k == "MANPATH" {
        return EnvCategory::Path;
    }

    // Language/Runtime
    if k.starts_with("PYTHON") || k.starts_with("NODE") || k.starts_with("NPM")
        || k.starts_with("RUBY") || k.starts_with("GEM") || k.starts_with("CARGO")
        || k.starts_with("RUST") || k.starts_with("GO") || k.starts_with("JAVA")
        || k.starts_with("NVM") || k.starts_with("RBENV") || k.starts_with("PYENV")
        || k.starts_with("VIRTUAL_ENV") || k.starts_with("CONDA")
        || k == "CC" || k == "CXX" || k == "CFLAGS" || k == "LDFLAGS"
    {
        return EnvCategory::Language;
    }

    // Terminal
    if k.starts_with("TERM") || k.starts_with("DISPLAY") || k.starts_with("WAYLAND")
        || k.starts_with("XDG") || k == "COLORTERM" || k == "COLUMNS" || k == "LINES"
    {
        return EnvCategory::Terminal;
    }

    // Editor
    if k == "EDITOR" || k == "VISUAL" || k == "PAGER" || k.starts_with("LESS")
        || k.starts_with("VIM") || k.starts_with("NVIM")
    {
        return EnvCategory::Editor;
    }

    // Git
    if k.starts_with("GIT") || k == "GPG_TTY" {
        return EnvCategory::Git;
    }

    // Cloud/DevOps
    if k.starts_with("AWS") || k.starts_with("AZURE") || k.starts_with("GCP")
        || k.starts_with("GOOGLE") || k.starts_with("DOCKER") || k.starts_with("KUBE")
        || k.starts_with("TERRAFORM") || k.starts_with("VAULT")
        || k.starts_with("CI") || k.starts_with("GITHUB") || k.starts_with("GITLAB")
    {
        return EnvCategory::Cloud;
    }

    EnvCategory::Other
}

/// Check if a variable likely contains a secret
pub fn is_secret(key: &str, value: &str) -> bool {
    let k = key.to_uppercase();
    // Key-based detection
    if k.contains("SECRET") || k.contains("TOKEN") || k.contains("PASSWORD")
        || k.contains("API_KEY") || k.contains("APIKEY") || k.contains("PRIVATE_KEY")
        || k.contains("CREDENTIAL") || k.contains("AUTH")
    {
        return true;
    }
    // Value-based detection
    if value.starts_with("sk-") || value.starts_with("ghp_") || value.starts_with("AKIA")
        || value.starts_with("xoxb-") || value.starts_with("xoxp-")
    {
        return true;
    }
    false
}

/// Mask a secret value for display
pub fn mask_value(value: &str) -> String {
    if value.len() <= 4 {
        "****".to_string()
    } else {
        let visible = &value[..4];
        format!("{}****", visible)
    }
}

/// Get all environment variables, categorized and sorted
pub fn get_all_env_vars() -> Vec<EnvVar> {
    let mut vars: Vec<EnvVar> = std::env::vars()
        .map(|(key, value)| {
            let category = categorize(&key);
            let secret = is_secret(&key, &value);
            EnvVar {
                key,
                value,
                category,
                is_secret: secret,
            }
        })
        .collect();

    vars.sort_by(|a, b| {
        a.category.name().cmp(b.category.name())
            .then(a.key.cmp(&b.key))
    });

    vars
}

/// Search environment variables
pub fn search_env(query: &str) -> Vec<EnvVar> {
    let q = query.to_lowercase();
    get_all_env_vars()
        .into_iter()
        .filter(|v| {
            v.key.to_lowercase().contains(&q) || v.value.to_lowercase().contains(&q)
        })
        .collect()
}

/// Get env vars grouped by category
pub fn grouped_env_vars() -> BTreeMap<String, Vec<EnvVar>> {
    let mut groups: BTreeMap<String, Vec<EnvVar>> = BTreeMap::new();
    for var in get_all_env_vars() {
        groups
            .entry(var.category.name().to_string())
            .or_default()
            .push(var);
    }
    groups
}

/// Get PATH entries as a list
pub fn path_entries() -> Vec<String> {
    std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .map(|s| s.to_string())
        .collect()
}

/// Check which PATH entries actually exist
pub fn validate_path_entries() -> Vec<(String, bool)> {
    path_entries()
        .into_iter()
        .map(|p| {
            let exists = std::path::Path::new(&p).exists();
            (p, exists)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_shell() {
        assert_eq!(categorize("SHELL"), EnvCategory::Shell);
        assert_eq!(categorize("HOME"), EnvCategory::Shell);
        assert_eq!(categorize("USER"), EnvCategory::Shell);
    }

    #[test]
    fn test_categorize_path() {
        assert_eq!(categorize("PATH"), EnvCategory::Path);
        assert_eq!(categorize("GOPATH"), EnvCategory::Path);
        assert_eq!(categorize("XDG_DATA_DIR"), EnvCategory::Path);
    }

    #[test]
    fn test_categorize_language() {
        assert_eq!(categorize("CARGO_HOME"), EnvCategory::Language);
        assert_eq!(categorize("NODE_ENV"), EnvCategory::Language);
        assert_eq!(categorize("VIRTUAL_ENV"), EnvCategory::Language);
    }

    #[test]
    fn test_is_secret() {
        assert!(is_secret("AWS_SECRET_ACCESS_KEY", "abc123"));
        assert!(is_secret("GITHUB_TOKEN", "ghp_abc123"));
        assert!(is_secret("ANYTHING", "sk-live-test123"));
        assert!(!is_secret("PATH", "/usr/bin"));
        assert!(!is_secret("HOME", "/Users/test"));
    }

    #[test]
    fn test_mask_value() {
        assert_eq!(mask_value("sk-live-abc123def"), "sk-l****");
        assert_eq!(mask_value("ab"), "****");
    }

    #[test]
    fn test_get_all_env_vars() {
        let vars = get_all_env_vars();
        assert!(!vars.is_empty());
        // Should have at least PATH and HOME
        assert!(vars.iter().any(|v| v.key == "PATH"));
    }

    #[test]
    fn test_path_entries() {
        let entries = path_entries();
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_validate_path() {
        let validated = validate_path_entries();
        assert!(!validated.is_empty());
        // /usr/bin should exist
        assert!(validated.iter().any(|(p, exists)| p == "/usr/bin" && *exists));
    }
}
