use std::fs;
use std::path::PathBuf;
use chrono::Local;
use super::repository::Repository;
use super::status::StatusScanner;

#[derive(Debug, Clone)]
pub struct ShelfEntry {
    pub id: usize,
    pub message: String,
    pub timestamp: String,
    pub files: Vec<String>,
}

pub struct ShelfManager;

impl ShelfManager {
    fn shelf_dir(repo: &Repository) -> PathBuf {
        repo.git_dir.join("tachyon_shelf")
    }

    pub fn save(repo: &Repository, custom_msg: Option<&str>) -> Result<Option<usize>, String> {
        let status = StatusScanner::scan(repo)?;
        if !status.has_staged() && !status.has_unstaged() {
            return Ok(None);
        }

        let shelf_dir = Self::shelf_dir(repo);
        fs::create_dir_all(&shelf_dir).map_err(|e| e.to_string())?;

        let shelves = Self::list(repo)?;
        let new_id = shelves.len();

        let branch = repo.current_branch_name();
        let default_msg = format!("WIP on {}: changes", branch);
        let msg = custom_msg.unwrap_or(&default_msg);
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let entry_dir = shelf_dir.join(format!("{}", new_id));
        fs::create_dir_all(&entry_dir).map_err(|e| e.to_string())?;

        // Save metadata
        let meta = format!("{}\n{}\n", msg, now);
        fs::write(entry_dir.join("meta.txt"), meta).map_err(|e| e.to_string())?;

        // Back up modified files and clean them from worktree
        let mut modified_files = Vec::new();
        for path in status.staged_new.iter()
            .chain(status.staged_modified.iter())
            .chain(status.unstaged_modified.iter())
        {
            let full = repo.work_dir.join(path);
            if full.exists() {
                let backup_file = entry_dir.join("files").join(path);
                if let Some(p) = backup_file.parent() {
                    fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
                fs::copy(&full, &backup_file).map_err(|e| e.to_string())?;
                modified_files.push(path.clone());
            }
        }

        // Restore working tree to HEAD
        if let Some(head) = status.head_oid {
            let _ = super::checkout::CheckoutEngine::checkout_commit(repo, &head, None, true);
            // Reset index to head
            if let Ok(head_state) = repo.read_head() {
                if let super::repository::HeadState::Branch { name, .. } = head_state {
                    let _ = repo.set_head_branch(&name);
                }
            }
        }

        Ok(Some(new_id))
    }

    pub fn list(repo: &Repository) -> Result<Vec<ShelfEntry>, String> {
        let shelf_dir = Self::shelf_dir(repo);
        if !shelf_dir.exists() {
            return Ok(Vec::new());
        }

        let mut list = Vec::new();
        let entries = fs::read_dir(&shelf_dir).map_err(|e| e.to_string())?;

        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Ok(id) = name.parse::<usize>() {
                    let meta_file = entry.path().join("meta.txt");
                    if let Ok(content) = fs::read_to_string(meta_file) {
                        let mut lines = content.lines();
                        let message = lines.next().unwrap_or("").to_string();
                        let timestamp = lines.next().unwrap_or("").to_string();

                        let files_dir = entry.path().join("files");
                        let mut files = Vec::new();
                        if files_dir.exists() {
                            for f in walkdir::WalkDir::new(&files_dir).into_iter().flatten() {
                                if f.file_type().is_file() {
                                    if let Ok(rel) = f.path().strip_prefix(&files_dir) {
                                        files.push(rel.to_string_lossy().to_string());
                                    }
                                }
                            }
                        }

                        list.push(ShelfEntry {
                            id,
                            message,
                            timestamp,
                            files,
                        });
                    }
                }
            }
        }

        list.sort_by_key(|e| e.id);
        Ok(list)
    }

    pub fn apply(repo: &Repository, id: usize) -> Result<(), String> {
        let shelf_dir = Self::shelf_dir(repo);
        let entry_dir = shelf_dir.join(format!("{}", id));
        if !entry_dir.exists() {
            return Err(format!("Shelf entry #{} does not exist", id));
        }

        let files_dir = entry_dir.join("files");
        if files_dir.exists() {
            for entry in walkdir::WalkDir::new(&files_dir).into_iter().flatten() {
                if entry.file_type().is_file() {
                    let rel = entry.path().strip_prefix(&files_dir).map_err(|e| e.to_string())?;
                    let target = repo.work_dir.join(rel);
                    if let Some(p) = target.parent() {
                        fs::create_dir_all(p).map_err(|e| e.to_string())?;
                    }
                    fs::copy(entry.path(), &target).map_err(|e| e.to_string())?;
                }
            }
        }

        Ok(())
    }

    pub fn drop(repo: &Repository, id: usize) -> Result<(), String> {
        let shelf_dir = Self::shelf_dir(repo);
        let entry_dir = shelf_dir.join(format!("{}", id));
        if !entry_dir.exists() {
            return Err(format!("Shelf entry #{} does not exist", id));
        }
        fs::remove_dir_all(entry_dir).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn pop(repo: &Repository, id: usize) -> Result<(), String> {
        Self::apply(repo, id)?;
        Self::drop(repo, id)
    }
}
