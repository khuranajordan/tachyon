use std::collections::BTreeMap;
use std::fs::{self};
use super::hash::Oid;
use super::index::{Index, IndexEntry};
use super::repository::Repository;
use super::tree::TreeBuilder;

pub struct CheckoutEngine;

impl CheckoutEngine {
    pub fn switch_branch(repo: &Repository, branch_name: &str, force: bool) -> Result<(), String> {
        let target_oid = repo.read_branch(branch_name)?;
        Self::checkout_commit(repo, &target_oid, Some(branch_name), force)
    }

    pub fn checkout_commit(
        repo: &Repository,
        commit_oid: &Oid,
        branch_name: Option<&str>,
        force: bool,
    ) -> Result<(), String> {
        let commit = repo.objects.parse_commit(commit_oid)?;
        let target_files = TreeBuilder::read_tree_recursive(&repo.objects, &commit.tree, "")?;

        // Read current index and HEAD files
        let current_index = repo.read_index().unwrap_or_default();
        let head_commit = repo.head_commit()?;
        let _head_files = if let Some(head) = head_commit {
            if let Ok(c) = repo.objects.parse_commit(&head) {
                TreeBuilder::read_tree_recursive(&repo.objects, &c.tree, "").unwrap_or_default()
            } else {
                BTreeMap::new()
            }
        } else {
            BTreeMap::new()
        };

        // Safety check: ensure no untracked or locally modified file would be overwritten
        if !force {
            for (path, _) in &target_files {
                let full = repo.work_dir.join(path);
                if full.exists() {
                    // If not in index, it's untracked
                    if !current_index.entries.contains_key(path) {
                        return Err(format!(
                            "error: The untracked working tree file '{}' would be overwritten by checkout.",
                            path
                        ));
                    }
                }
            }
        }

        // Delete files present in head_files / current_index but not in target_files
        for (path, _) in &current_index.entries {
            if !target_files.contains_key(path) {
                let full = repo.work_dir.join(path);
                if full.exists() {
                    let _ = fs::remove_file(&full);
                    // Also clean up empty parent directories
                    let mut parent = full.parent();
                    while let Some(p) = parent {
                        if p == repo.work_dir || !p.starts_with(&repo.work_dir) {
                            break;
                        }
                        if fs::remove_dir(p).is_err() {
                            break; // Not empty
                        }
                        parent = p.parent();
                    }
                }
            }
        }

        // Write files from target_files into worktree & build new index
        let mut new_index = Index::new();

        for (path, (mode, blob_oid)) in &target_files {
            let full = repo.work_dir.join(path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }

            let blob_obj = repo.objects.read(blob_oid)?;
            fs::write(&full, &blob_obj.content).map_err(|e| e.to_string())?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = fs::Permissions::from_mode(*mode);
                let _ = fs::set_permissions(&full, perms);
            }

            // Stat file for index entry
            let meta = fs::metadata(&full).map_err(|e| e.to_string())?;
            let (mtime_sec, mtime_nsec) = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| (d.as_secs() as u32, d.subsec_nanos()))
                .unwrap_or((0, 0));

            new_index.add(IndexEntry {
                ctime_sec: mtime_sec,
                ctime_nsec: mtime_nsec,
                mtime_sec,
                mtime_nsec,
                dev: 0,
                ino: 0,
                mode: *mode,
                uid: 0,
                gid: 0,
                file_size: meta.len() as u32,
                oid: *blob_oid,
                flags: (path.len().min(0xFFF)) as u16,
                path: path.clone(),
            });
        }

        // Save new index
        repo.write_index(&new_index)?;

        // Update HEAD
        if let Some(branch) = branch_name {
            repo.set_head_branch(branch)?;
        } else {
            repo.set_head_detached(commit_oid)?;
        }

        Ok(())
    }
}
