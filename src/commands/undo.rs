use std::fs;
use crate::core::commit::Committer;
use crate::core::repository::Repository;
use crate::core::tree::TreeBuilder;

pub fn execute(commit_hex: String) -> Result<(), String> {
    let repo = Repository::find_root()?;
    let commit_oid = repo.resolve_oid(&commit_hex)?;
    let commit = repo.objects.parse_commit(&commit_oid)?;

    if commit.parents.is_empty() {
        return Err("Cannot undo a root commit with no parent".to_string());
    }

    let parent_oid = commit.parents[0];
    let parent_commit = repo.objects.parse_commit(&parent_oid)?;

    // We want to apply parent's state of modified files on top of current HEAD
    let parent_files = TreeBuilder::read_tree_recursive(&repo.objects, &parent_commit.tree, "")?;
    let commit_files = TreeBuilder::read_tree_recursive(&repo.objects, &commit.tree, "")?;

    let mut index = repo.read_index()?;

    for (path, (_, parent_blob_oid)) in &parent_files {
        if commit_files.get(path).map(|(_, o)| o) != Some(parent_blob_oid) {
            // This file was changed in the reverted commit -> restore to parent state
            let full = repo.work_dir.join(path);
            let blob_obj = repo.objects.read(parent_blob_oid)?;
            if let Some(p) = full.parent() {
                let _ = fs::create_dir_all(p);
            }
            fs::write(&full, &blob_obj.content).map_err(|e| e.to_string())?;

            let meta = fs::metadata(&full).map_err(|e| e.to_string())?;
            index.add(crate::core::index::IndexEntry {
                ctime_sec: 0,
                ctime_nsec: 0,
                mtime_sec: 0,
                mtime_nsec: 0,
                dev: 0,
                ino: 0,
                mode: 0o100644,
                uid: 0,
                gid: 0,
                file_size: meta.len() as u32,
                oid: *parent_blob_oid,
                flags: (path.len().min(0xFFF)) as u16,
                path: path.clone(),
            });
        }
    }

    // Check for files added in commit that didn't exist in parent -> remove them
    for path in commit_files.keys() {
        if !parent_files.contains_key(path) {
            let full = repo.work_dir.join(path);
            if full.exists() {
                let _ = fs::remove_file(&full);
            }
            index.remove(path);
        }
    }

    repo.write_index(&index)?;

    let revert_msg = format!("Undo \"{}\"\n\nThis reverts commit {}.", commit.message.lines().next().unwrap_or(""), commit_oid.to_hex());
    let new_commit_oid = Committer::commit(&repo, &revert_msg, None, &[])?;

    println!(
        "\x1b[1;32m✓\x1b[0m Created revert commit \x1b[33m{}\x1b[0m undoing {}.",
        new_commit_oid.short(), commit_oid.short()
    );

    Ok(())
}
