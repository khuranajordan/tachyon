use std::fs;
use crate::core::commit::Committer;
use crate::core::repository::Repository;
use crate::core::tree::TreeBuilder;

pub fn execute(commit_hex: String) -> Result<(), String> {
    let repo = Repository::find_root()?;
    let commit_oid = repo.resolve_oid(&commit_hex)?;
    let commit = repo.objects.parse_commit(&commit_oid)?;

    let commit_files = TreeBuilder::read_tree_recursive(&repo.objects, &commit.tree, "")?;
    let parent_files = if !commit.parents.is_empty() {
        let parent_commit = repo.objects.parse_commit(&commit.parents[0])?;
        TreeBuilder::read_tree_recursive(&repo.objects, &parent_commit.tree, "")?
    } else {
        std::collections::BTreeMap::new()
    };

    let mut index = repo.read_index()?;
    let mut modified_count = 0;

    for (path, (mode, blob_oid)) in &commit_files {
        let parent_blob_oid = parent_files.get(path).map(|(_, o)| *o);
        if Some(*blob_oid) != parent_blob_oid {
            let full = repo.work_dir.join(path);
            let blob_obj = repo.objects.read(blob_oid)?;
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
                mode: *mode,
                uid: 0,
                gid: 0,
                file_size: meta.len() as u32,
                oid: *blob_oid,
                flags: (path.len().min(0xFFF)) as u16,
                path: path.clone(),
            });
            modified_count += 1;
        }
    }

    repo.write_index(&index)?;

    let msg = format!("{}\n(plucked from commit {})", commit.message, commit_oid.to_hex());
    let new_commit_oid = Committer::commit(&repo, &msg, None, &[])?;

    println!(
        "\x1b[1;32m✓\x1b[0m Plucked commit \x1b[33m{}\x1b[0m into new commit \x1b[32m{}\x1b[0m ({} files modified).",
        commit_oid.short(), new_commit_oid.short(), modified_count
    );

    Ok(())
}
