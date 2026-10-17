use std::collections::BTreeMap;
use crate::core::hash::Oid;
use crate::core::index::IndexEntry;
use crate::core::repository::Repository;
use crate::core::tree::TreeBuilder;

pub fn execute(paths: Vec<String>, all: bool) -> Result<(), String> {
    let repo = Repository::find_root()?;
    let mut index = repo.read_index()?;

    let head_files: BTreeMap<String, (u32, Oid)> = if let Some(head) = repo.head_commit()? {
        if let Ok(c) = repo.objects.parse_commit(&head) {
            TreeBuilder::read_tree_recursive(&repo.objects, &c.tree, "").unwrap_or_default()
        } else {
            BTreeMap::new()
        }
    } else {
        BTreeMap::new()
    };

    let target_paths = if all || paths.contains(&".".to_string()) || paths.is_empty() {
        index.entries.keys().cloned().collect()
    } else {
        paths
    };

    let mut unstage_count = 0;

    for path in target_paths {
        if index.entries.contains_key(&path) {
            if let Some((mode, head_oid)) = head_files.get(&path) {
                // Restore index entry from HEAD
                index.add(IndexEntry {
                    ctime_sec: 0,
                    ctime_nsec: 0,
                    mtime_sec: 0,
                    mtime_nsec: 0,
                    dev: 0,
                    ino: 0,
                    mode: *mode,
                    uid: 0,
                    gid: 0,
                    file_size: 0,
                    oid: *head_oid,
                    flags: (path.len().min(0xFFF)) as u16,
                    path: path.clone(),
                });
            } else {
                // Was newly staged, remove from index
                index.remove(&path);
            }
            unstage_count += 1;
        }
    }

    repo.write_index(&index)?;
    println!("\x1b[1;32m✓\x1b[0m Unstaged \x1b[1;36m{}\x1b[0m file(s).", unstage_count);
    Ok(())
}
