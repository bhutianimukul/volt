use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::SystemTime;

/// A recorded command execution
#[derive(Debug, Clone)]
pub struct SessionEntry {
    pub id: usize,
    pub command: String,
    pub exit_code: Option<i32>,
    pub working_dir: PathBuf,
    pub timestamp: SystemTime,
    pub duration_ms: Option<u64>,
    pub output_preview: String, // First 500 chars of output
}

/// Session recorder — append-only structured log of all commands
#[derive(Debug)]
pub struct SessionRecorder {
    entries: VecDeque<SessionEntry>,
    next_id: usize,
    max_entries: usize,
}

impl SessionRecorder {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            next_id: 0,
            max_entries: 10_000,
        }
    }

    pub fn record(&mut self, command: String, working_dir: PathBuf) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        self.entries.push_back(SessionEntry {
            id,
            command,
            exit_code: None,
            working_dir,
            timestamp: SystemTime::now(),
            duration_ms: None,
            output_preview: String::new(),
        });

        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }

        id
    }

    pub fn complete(
        &mut self,
        id: usize,
        exit_code: i32,
        duration_ms: u64,
        output_preview: String,
    ) {
        if let Some(entry) = self.entries.iter_mut().rev().find(|e| e.id == id) {
            entry.exit_code = Some(exit_code);
            entry.duration_ms = Some(duration_ms);
            entry.output_preview = if output_preview.len() > 500 {
                output_preview[..500].to_string()
            } else {
                output_preview
            };
        }
    }

    /// Search for entries matching a query
    pub fn search(&self, query: &str) -> Vec<&SessionEntry> {
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                e.command.to_lowercase().contains(&q)
                    || e.output_preview.to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Find all failed commands
    pub fn failed_commands(&self) -> Vec<&SessionEntry> {
        self.entries
            .iter()
            .filter(|e| matches!(e.exit_code, Some(code) if code != 0))
            .collect()
    }

    /// Get the last N entries
    pub fn recent(&self, count: usize) -> Vec<&SessionEntry> {
        self.entries.iter().rev().take(count).collect()
    }

    /// Get all entries
    pub fn all(&self) -> &VecDeque<SessionEntry> {
        &self.entries
    }

    /// Export session as a simple text format
    pub fn export_text(&self) -> String {
        let mut out = String::new();
        for entry in &self.entries {
            let status = match entry.exit_code {
                Some(0) => "\u{2713}",
                Some(_) => "\u{2717}",
                None => "?",
            };
            let dur = entry
                .duration_ms
                .map(|d| format!("{}ms", d))
                .unwrap_or_default();
            out.push_str(&format!(
                "[{}] {} {} ({})\n",
                status,
                entry.command,
                dur,
                entry.working_dir.display()
            ));
        }
        out
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for SessionRecorder {
    fn default() -> Self {
        Self::new()
    }
}
