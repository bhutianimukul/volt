use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const MAX_CHECKPOINTS: usize = 50;
#[allow(dead_code)]
const MAX_TOTAL_SIZE_BYTES: u64 = 5 * 1024 * 1024 * 1024; // 5GB

#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub id: usize,
    pub name: Option<String>,
    pub command: String,
    pub working_dir: PathBuf,
    pub created_at: SystemTime,
    pub snapshot_dir: PathBuf,
    pub files: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct UndoManager {
    checkpoints: VecDeque<Checkpoint>,
    next_id: usize,
    storage_dir: PathBuf,
}

impl UndoManager {
    pub fn new() -> Self {
        let storage_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("volt")
            .join("checkpoints");

        // Create storage directory
        let _ = fs::create_dir_all(&storage_dir);

        Self {
            checkpoints: VecDeque::new(),
            next_id: 0,
            storage_dir,
        }
    }

    /// Create a checkpoint before a command executes.
    /// Snapshots the specified files using APFS clonefile (copy-on-write).
    pub fn create_checkpoint(
        &mut self,
        command: &str,
        working_dir: &Path,
        files_to_snapshot: &[PathBuf],
    ) -> Option<usize> {
        let id = self.next_id;
        self.next_id += 1;

        let snapshot_dir = self.storage_dir.join(format!("checkpoint_{}", id));
        if fs::create_dir_all(&snapshot_dir).is_err() {
            tracing::warn!("Failed to create checkpoint directory");
            return None;
        }

        let mut snapshotted_files = Vec::new();

        for file in files_to_snapshot {
            let abs_path = if file.is_absolute() {
                file.clone()
            } else {
                working_dir.join(file)
            };

            if !abs_path.exists() {
                continue;
            }

            // Create snapshot using APFS clonefile (macOS) or regular copy
            let relative = file.strip_prefix(working_dir).unwrap_or(file);
            let dest = snapshot_dir.join(relative);

            if let Some(parent) = dest.parent() {
                let _ = fs::create_dir_all(parent);
            }

            if clone_or_copy(&abs_path, &dest) {
                snapshotted_files.push(abs_path);
            }
        }

        if snapshotted_files.is_empty() {
            let _ = fs::remove_dir_all(&snapshot_dir);
            return None;
        }

        let checkpoint = Checkpoint {
            id,
            name: None,
            command: command.to_string(),
            working_dir: working_dir.to_path_buf(),
            created_at: SystemTime::now(),
            snapshot_dir,
            files: snapshotted_files,
        };

        self.checkpoints.push_back(checkpoint);

        // Enforce max checkpoints
        while self.checkpoints.len() > MAX_CHECKPOINTS {
            if let Some(old) = self.checkpoints.pop_front() {
                let _ = fs::remove_dir_all(&old.snapshot_dir);
            }
        }

        Some(id)
    }

    /// Undo the last checkpoint — restore files to their pre-command state.
    pub fn undo_last(&mut self) -> Option<String> {
        let checkpoint = self.checkpoints.pop_back()?;
        let mut restored = Vec::new();

        for file in &checkpoint.files {
            let relative = file.strip_prefix(&checkpoint.working_dir).unwrap_or(file);
            let snapshot_path = checkpoint.snapshot_dir.join(relative);

            if snapshot_path.exists() {
                // Restore the file
                if let Some(parent) = file.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if fs::copy(&snapshot_path, file).is_ok() {
                    restored.push(file.display().to_string());
                }
            }
        }

        // Clean up snapshot
        let _ = fs::remove_dir_all(&checkpoint.snapshot_dir);

        if restored.is_empty() {
            None
        } else {
            Some(format!(
                "Undid '{}': restored {} file(s)",
                checkpoint.command,
                restored.len()
            ))
        }
    }

    /// Name the most recent checkpoint.
    pub fn name_last(&mut self, name: String) {
        if let Some(checkpoint) = self.checkpoints.back_mut() {
            checkpoint.name = Some(name);
        }
    }

    /// List all checkpoints.
    #[allow(dead_code)]
    pub fn list(&self) -> Vec<&Checkpoint> {
        self.checkpoints.iter().collect()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.checkpoints.len()
    }
}

/// Clone a file using APFS clonefile on macOS, falling back to a regular copy.
/// Returns `true` on success.
fn clone_or_copy(src: &Path, dest: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::CString;

        let src_c = match CString::new(src.to_string_lossy().as_bytes()) {
            Ok(c) => c,
            Err(_) => return fallback_copy(src, dest),
        };
        let dst_c = match CString::new(dest.to_string_lossy().as_bytes()) {
            Ok(c) => c,
            Err(_) => return fallback_copy(src, dest),
        };

        // libc::clonefile is available on macOS — zero-cost COW on APFS
        let result = unsafe { libc::clonefile(src_c.as_ptr(), dst_c.as_ptr(), 0) };

        if result == 0 {
            return true;
        }

        // Fallback to regular copy (e.g. non-APFS filesystem)
        fallback_copy(src, dest)
    }

    #[cfg(not(target_os = "macos"))]
    {
        fallback_copy(src, dest)
    }
}

fn fallback_copy(src: &Path, dest: &Path) -> bool {
    fs::copy(src, dest).is_ok()
}
