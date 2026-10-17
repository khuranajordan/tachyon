use std::collections::HashSet;
use std::fs;
use super::checkout::CheckoutEngine;
use super::commit::Committer;
use super::hash::Oid;
use super::repository::Repository;
use super::tree::TreeBuilder;

pub struct MergeEngine;

pub enum MergeResult {
    FastForward(Oid),
    AlreadyUpToDate,
    Merged(Oid),
    Conflict(Vec<String>),
}

impl MergeEngine {
    pub fn merge(repo: &Repository, target_branch: &str) -> Result<MergeResult, String> {
        let current_branch = repo.current_branch_name();
        let head_commit_oid = repo.head_commit()?
            .ok_or_else(|| "Current branch has no commits".to_string())?;
        let target_commit_oid = repo.read_branch(target_branch)?;

        if head_commit_oid == target_commit_oid {
            return Ok(MergeResult::AlreadyUpToDate);
        }

        // Check if fast-forward is possible (is head_commit an ancestor of target_commit?)
        if Self::is_ancestor(repo, &head_commit_oid, &target_commit_oid)? {
            // Fast-forward
            CheckoutEngine::checkout_commit(repo, &target_commit_oid, Some(&current_branch), true)?;
            repo.write_branch(&current_branch, &target_commit_oid)?;
            return Ok(MergeResult::FastForward(target_commit_oid));
        }

        // Check if target is already merged (target_commit is ancestor of head_commit)
        if Self::is_ancestor(repo, &target_commit_oid, &head_commit_oid)? {
            return Ok(MergeResult::AlreadyUpToDate);
        }

        // 3-way merge
        let base_commit_oid = Self::find_merge_base(repo, &head_commit_oid, &target_commit_oid)?
            .ok_or_else(|| "No common ancestor found for merge".to_string())?;

        let base_commit = repo.objects.parse_commit(&base_commit_oid)?;
        let head_commit = repo.objects.parse_commit(&head_commit_oid)?;
        let target_commit = repo.objects.parse_commit(&target_commit_oid)?;

        let base_files = TreeBuilder::read_tree_recursive(&repo.objects, &base_commit.tree, "")?;
        let head_files = TreeBuilder::read_tree_recursive(&repo.objects, &head_commit.tree, "")?;
        let target_files = TreeBuilder::read_tree_recursive(&repo.objects, &target_commit.tree, "")?;

        let mut all_paths = HashSet::new();
        for k in base_files.keys() { all_paths.insert(k.clone()); }
        for k in head_files.keys() { all_paths.insert(k.clone()); }
        for k in target_files.keys() { all_paths.insert(k.clone()); }

        let mut conflicts = Vec::new();
        let mut index = repo.read_index().unwrap_or_default();

        for path in all_paths {
            let base = base_files.get(&path).map(|(_, oid)| *oid);
            let head = head_files.get(&path).map(|(_, oid)| *oid);
            let target = target_files.get(&path).map(|(_, oid)| *oid);

            if head == target {
                // Both made the same change or neither changed
                continue;
            }

            if base == head && base != target {
                // Target made change, HEAD did not -> apply target
                match target {
                    Some(oid) => {
                        let full = repo.work_dir.join(&path);
                        if let Some(p) = full.parent() {
                            fs::create_dir_all(p).map_err(|e| e.to_string())?;
                        }
                        let obj = repo.objects.read(&oid)?;
                        fs::write(&full, &obj.content).map_err(|e| e.to_string())?;

                        let meta = fs::metadata(&full).map_err(|e| e.to_string())?;
                        index.add(super::index::IndexEntry {
                            ctime_sec: 0,
                            ctime_nsec: 0,
                            mtime_sec: 0,
                            mtime_nsec: 0,
                            dev: 0,
                            ino: 0,
                            mode: target_files.get(&path).map(|(m, _)| *m).unwrap_or(0o100644),
                            uid: 0,
                            gid: 0,
                            file_size: meta.len() as u32,
                            oid,
                            flags: (path.len().min(0xFFF)) as u16,
                            path: path.clone(),
                        });
                    }
                    None => {
                        // Deleted in target
                        let full = repo.work_dir.join(&path);
                        if full.exists() {
                            let _ = fs::remove_file(&full);
                        }
                        index.remove(&path);
                    }
                }
            } else if base != head && base == target {
                // Head changed, target did not -> keep head
                continue;
            } else {
                // Conflict! Both changed differently
                conflicts.push(path);
            }
        }

        if !conflicts.is_empty() {
            repo.write_index(&index)?;
            return Ok(MergeResult::Conflict(conflicts));
        }

        repo.write_index(&index)?;

        let merge_msg = format!("Merge branch '{}' into {}", target_branch, current_branch);
        let commit_oid = Committer::commit(repo, &merge_msg, None, &[target_commit_oid])?;

        Ok(MergeResult::Merged(commit_oid))
    }

    fn is_ancestor(repo: &Repository, ancestor: &Oid, descendant: &Oid) -> Result<bool, String> {
        let mut visited = HashSet::new();
        let mut queue = vec![*descendant];

        while let Some(curr) = queue.pop() {
            if curr == *ancestor {
                return Ok(true);
            }
            if visited.insert(curr) {
                if let Ok(c) = repo.objects.parse_commit(&curr) {
                    for parent in c.parents {
                        queue.push(parent);
                    }
                }
            }
        }

        Ok(false)
    }

    fn find_merge_base(repo: &Repository, a: &Oid, b: &Oid) -> Result<Option<Oid>, String> {
        let mut ancestors_a = HashSet::new();
        let mut queue_a = vec![*a];

        while let Some(curr) = queue_a.pop() {
            if ancestors_a.insert(curr) {
                if let Ok(c) = repo.objects.parse_commit(&curr) {
                    for parent in c.parents {
                        queue_a.push(parent);
                    }
                }
            }
        }

        let mut queue_b = vec![*b];
        let mut visited_b = HashSet::new();

        while let Some(curr) = queue_b.pop() {
            if ancestors_a.contains(&curr) {
                return Ok(Some(curr));
            }
            if visited_b.insert(curr) {
                if let Ok(c) = repo.objects.parse_commit(&curr) {
                    for parent in c.parents {
                        queue_b.push(parent);
                    }
                }
            }
        }

        Ok(None)
    }
}
