//! Bookmark system — save and recall important commands and their context.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: usize,
    pub name: Option<String>,
    pub command: String,
    pub output_preview: String,
    pub working_dir: PathBuf,
    pub exit_code: Option<i32>,
    pub created_at: SystemTime,
    pub tags: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BookmarkStore {
    bookmarks: BTreeMap<usize, Bookmark>,
    next_id: usize,
}

impl BookmarkStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a bookmark
    pub fn add(
        &mut self,
        command: String,
        output_preview: String,
        working_dir: PathBuf,
        exit_code: Option<i32>,
    ) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.bookmarks.insert(
            id,
            Bookmark {
                id,
                name: None,
                command,
                output_preview: if output_preview.len() > 1000 {
                    output_preview[..1000].to_string()
                } else {
                    output_preview
                },
                working_dir,
                exit_code,
                created_at: SystemTime::now(),
                tags: Vec::new(),
            },
        );
        id
    }

    /// Name a bookmark
    pub fn set_name(&mut self, id: usize, name: String) {
        if let Some(bm) = self.bookmarks.get_mut(&id) {
            bm.name = Some(name);
        }
    }

    /// Add a tag to a bookmark
    pub fn add_tag(&mut self, id: usize, tag: String) {
        if let Some(bm) = self.bookmarks.get_mut(&id) {
            if !bm.tags.contains(&tag) {
                bm.tags.push(tag);
            }
        }
    }

    /// Remove a bookmark
    pub fn remove(&mut self, id: usize) -> bool {
        self.bookmarks.remove(&id).is_some()
    }

    /// Get a bookmark by id
    pub fn get(&self, id: usize) -> Option<&Bookmark> {
        self.bookmarks.get(&id)
    }

    /// List all bookmarks, newest first
    pub fn list(&self) -> Vec<&Bookmark> {
        let mut bms: Vec<&Bookmark> = self.bookmarks.values().collect();
        bms.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        bms
    }

    /// Search bookmarks by command or name
    pub fn search(&self, query: &str) -> Vec<&Bookmark> {
        let q = query.to_lowercase();
        self.bookmarks
            .values()
            .filter(|bm| {
                bm.command.to_lowercase().contains(&q)
                    || bm.name.as_deref().unwrap_or("").to_lowercase().contains(&q)
                    || bm.tags.iter().any(|t| t.to_lowercase().contains(&q))
                    || bm.output_preview.to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Search by tag
    pub fn by_tag(&self, tag: &str) -> Vec<&Bookmark> {
        let t = tag.to_lowercase();
        self.bookmarks
            .values()
            .filter(|bm| bm.tags.iter().any(|bt| bt.to_lowercase() == t))
            .collect()
    }

    /// Get failed command bookmarks
    pub fn failed(&self) -> Vec<&Bookmark> {
        self.bookmarks
            .values()
            .filter(|bm| matches!(bm.exit_code, Some(code) if code != 0))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.bookmarks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bookmarks.is_empty()
    }

    /// Save bookmarks to disk
    pub fn save(&self) -> Result<(), String> {
        let path = bookmarks_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
    }

    /// Load bookmarks from disk
    pub fn load() -> Self {
        let path = bookmarks_path();
        if !path.exists() {
            return Self::new();
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::new(),
        }
    }
}

fn bookmarks_path() -> PathBuf {
    rio_backend::config::config_dir_path().join("bookmarks.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get() {
        let mut store = BookmarkStore::new();
        let id = store.add(
            "ls -la".into(),
            "total 42\n...".into(),
            PathBuf::from("/tmp"),
            Some(0),
        );
        let bm = store.get(id).unwrap();
        assert_eq!(bm.command, "ls -la");
        assert_eq!(bm.exit_code, Some(0));
    }

    #[test]
    fn test_name_and_tag() {
        let mut store = BookmarkStore::new();
        let id = store.add("deploy".into(), "".into(), PathBuf::from("/app"), Some(0));
        store.set_name(id, "prod deploy".into());
        store.add_tag(id, "deploy".into());
        store.add_tag(id, "production".into());

        let bm = store.get(id).unwrap();
        assert_eq!(bm.name, Some("prod deploy".into()));
        assert_eq!(bm.tags.len(), 2);
    }

    #[test]
    fn test_search() {
        let mut store = BookmarkStore::new();
        store.add(
            "cargo test".into(),
            "155 passed".into(),
            PathBuf::from("/proj"),
            Some(0),
        );
        store.add(
            "npm install".into(),
            "added 500 packages".into(),
            PathBuf::from("/web"),
            Some(0),
        );

        let results = store.search("cargo");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].command, "cargo test");
    }

    #[test]
    fn test_by_tag() {
        let mut store = BookmarkStore::new();
        let id1 = store.add("cmd1".into(), "".into(), PathBuf::from("/"), Some(0));
        let _id2 = store.add("cmd2".into(), "".into(), PathBuf::from("/"), Some(0));
        store.add_tag(id1, "important".into());

        assert_eq!(store.by_tag("important").len(), 1);
        assert_eq!(store.by_tag("nonexistent").len(), 0);
    }

    #[test]
    fn test_failed() {
        let mut store = BookmarkStore::new();
        store.add("good".into(), "".into(), PathBuf::from("/"), Some(0));
        store.add("bad".into(), "error".into(), PathBuf::from("/"), Some(1));

        let failed = store.failed();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].command, "bad");
    }

    #[test]
    fn test_remove() {
        let mut store = BookmarkStore::new();
        let id = store.add("test".into(), "".into(), PathBuf::from("/"), Some(0));
        assert_eq!(store.len(), 1);
        assert!(store.remove(id));
        assert_eq!(store.len(), 0);
    }
}
