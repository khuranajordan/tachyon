use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use rayon::prelude::*;
use walkdir::WalkDir;
use super::hash::Oid;
use super::ignore::GitIgnore;
use super::repository::Repository;
use super::tree::TreeBuilder;

#[derive(Debug, Clone, Default)]
pub struct RepoStatus {
    pub branch: String,
    pub head_oid: Option<Oid>,
    pub staged_new: Vec<String>,
    pub staged_modified: Vec<String>,
    pub staged_deleted: Vec<String>,
    pub unstaged_modified: Vec<String>,
    pub unstaged_deleted: Vec<String>,
    pub untracked: Vec<String>,
}

impl RepoStatus {
    pub fn is_clean(&self) -> bool {
        self.staged_new.is_empty()
            && self.staged_modified.is_empty()
            && self.staged_deleted.is_empty()
            && self.unstaged_modified.is_empty()
            && self.unstaged_deleted.is_empty()
            && self.untracked.is_empty()
    }

    pub fn has_staged(&self) -> bool {
        !self.staged_new.is_empty()
            || !self.staged_modified.is_empty()
            || !self.staged_deleted.is_empty()
    }

    pub fn has_unstaged(&self) -> bool {
        !self.unstaged_modified.is_empty()
            || !self.unstaged_deleted.is_empty()
            || !self.untracked.is_empty()
    }
}

pub struct StatusScanner;

impl StatusScanner {
    pub fn scan(repo: &Repository) -> Result<RepoStatus, String> {
        let index = repo.read_index().unwrap_or_default();
        let head_commit_oid = repo.head_commit().unwrap_or(None);
        let branch = repo.current_branch_name();

        // 1. Get HEAD tree files
        let head_files: BTreeMap<String, (u32, Oid)> = if let Some(commit_oid) = head_commit_oid {
            if let Ok(commit_obj) = repo.objects.parse_commit(&commit_oid) {
                TreeBuilder::read_tree_recursive(&repo.objects, &commit_obj.tree, "")
                    .unwrap_or_default()
            } else {
                BTreeMap::new()
            }
        } else {
            BTreeMap::new()
        };

        // 2. Compare Index vs HEAD (staged changes)
        let mut staged_new = Vec::new();
        let mut staged_modified = Vec::new();
        let mut staged_deleted = Vec::new();

        for (path, entry) in &index.entries {
            match head_files.get(path) {
                Some((_, head_oid)) => {
                    if entry.oid != *head_oid {
                        staged_modified.push(path.clone());
                    }
                }
                None => {
                    staged_new.push(path.clone());
                }
            }
        }

        for path in head_files.keys() {
            if !index.entries.contains_key(path) {
                staged_deleted.push(path.clone());
            }
        }

        // 3. Scan Working Directory in parallel with rayon
        let gitignore = GitIgnore::new(&repo.work_dir);
        let work_dir = &repo.work_dir;

        // Collect all work tree files
        let walk_entries: Vec<_> = WalkDir::new(work_dir)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy();
                if name == ".git" {
                    return false;
                }
                if let Ok(rel) = entry.path().strip_prefix(work_dir) {
                    let rel_str = rel.to_string_lossy();
                    if !rel_str.is_empty() && gitignore.is_ignored(&rel_str, entry.file_type().is_dir()) {
                        return false;
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();

        // Map collected relative paths
        let worktree_files: Vec<String> = walk_entries
            .par_iter()
            .filter_map(|entry| {
                entry.path().strip_prefix(work_dir).ok().map(|rel| {
                    rel.to_string_lossy().replace('\\', "/")
                })
            })
            .collect();

        let worktree_set: BTreeSet<String> = worktree_files.iter().cloned().collect();

        // 4. Check for unstaged modifications in parallel across CPU cores
        let unstaged_modified: Vec<String> = index
            .entries
            .par_iter()
            .filter_map(|(path, entry)| {
                let full_path = work_dir.join(path);
                if !full_path.exists() {
                    return None; // Will be caught as unstaged_deleted
                }

                // Fast stat check: size
                if let Ok(meta) = fs::metadata(&full_path) {
                    if meta.len() as u32 != entry.file_size {
                        return Some(path.clone());
                    }

                    // If mtime differs, compute hash to verify
                    if let Ok(modified) = meta.modified() {
                        if let Ok(dur) = modified.duration_since(std::time::UNIX_EPOCH) {
                            let mtime_sec = dur.as_secs() as u32;
                            if mtime_sec != entry.mtime_sec {
                                if let Ok(bytes) = fs::read(&full_path) {
                                    let current_oid = Oid::hash_object("blob", &bytes);
                                    if current_oid != entry.oid {
                                        return Some(path.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                None
            })
            .collect();

        // 5. Check for unstaged deletions
        let mut unstaged_deleted = Vec::new();
        for path in index.entries.keys() {
            if !worktree_set.contains(path) {
                unstaged_deleted.push(path.clone());
            }
        }

        // 6. Check for untracked files
        let untracked: Vec<String> = worktree_files
            .into_par_iter()
            .filter(|path| !index.entries.contains_key(path))
            .collect();

        let mut staged_new = staged_new;
        staged_new.sort();
        let mut staged_modified = staged_modified;
        staged_modified.sort();
        let mut staged_deleted = staged_deleted;
        staged_deleted.sort();
        let mut unstaged_modified = unstaged_modified;
        unstaged_modified.sort();
        let mut unstaged_deleted = unstaged_deleted;
        unstaged_deleted.sort();
        let mut untracked = untracked;
        untracked.sort();

        Ok(RepoStatus {
            branch,
            head_oid: head_commit_oid,
            staged_new,
            staged_modified,
            staged_deleted,
            unstaged_modified,
            unstaged_deleted,
            untracked,
        })
    }
}
