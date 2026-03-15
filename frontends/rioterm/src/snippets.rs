use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub name: String,
    pub command: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SnippetStore {
    pub snippets: BTreeMap<String, Snippet>,
}

impl SnippetStore {
    pub fn load() -> Self {
        let path = rio_backend::config::config_dir_path().join("snippets.toml");
        if !path.exists() {
            // Create default with examples
            let default = Self::default_snippets();
            if let Ok(toml) = toml::to_string_pretty(&default) {
                let _ = std::fs::write(&path, toml);
            }
            return default;
        }
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|c| toml::from_str(&c).ok())
            .unwrap_or_default()
    }

    fn default_snippets() -> Self {
        let mut snippets = BTreeMap::new();
        snippets.insert(
            "docker-ps".into(),
            Snippet {
                name: "Docker containers".into(),
                command:
                    "docker ps --format 'table {{.Names}}\\t{{.Status}}\\t{{.Ports}}'"
                        .into(),
                description: Some("List running Docker containers".into()),
                tags: vec!["docker".into()],
            },
        );
        snippets.insert(
            "git-log".into(),
            Snippet {
                name: "Git log pretty".into(),
                command: "git log --oneline --graph --decorate -20".into(),
                description: Some("Pretty git log with graph".into()),
                tags: vec!["git".into()],
            },
        );
        snippets.insert(
            "disk-usage".into(),
            Snippet {
                name: "Disk usage".into(),
                command: "du -sh * | sort -rh | head -20".into(),
                description: Some("Top 20 largest items in current directory".into()),
                tags: vec!["system".into()],
            },
        );
        Self { snippets }
    }

    pub fn search(&self, query: &str) -> Vec<&Snippet> {
        let q = query.to_lowercase();
        self.snippets
            .values()
            .filter(|s| {
                s.name.to_lowercase().contains(&q)
                    || s.command.to_lowercase().contains(&q)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_snippets() {
        let store = SnippetStore::default_snippets();
        assert!(store.snippets.contains_key("docker-ps"));
        assert!(store.snippets.contains_key("git-log"));
        assert!(store.snippets.contains_key("disk-usage"));
    }

    #[test]
    fn test_search() {
        let store = SnippetStore::default_snippets();
        let results = store.search("docker");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Docker containers");

        let results = store.search("git");
        assert_eq!(results.len(), 1);

        let results = store.search("system");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_empty() {
        let store = SnippetStore::default_snippets();
        let results = store.search("nonexistent");
        assert!(results.is_empty());
    }
}
