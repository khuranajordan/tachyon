use std::fs;
use crate::core::repository::Repository;
use crate::core::status::StatusScanner;

pub fn execute(paths: Vec<String>, all: bool) -> Result<(), String> {
    let repo = Repository::find_root()?;
    let index = repo.read_index()?;
    let status = StatusScanner::scan(&repo)?;

    let target_paths: Vec<String> = if all || paths.contains(&".".to_string()) {
        status.unstaged_modified.into_iter().chain(status.unstaged_deleted.into_iter()).collect()
    } else if !paths.is_empty() {
        paths
    } else {
        return Err("No files specified to discard. Use 'tachyon discard <file>...' or 'tachyon discard --all'".to_string());
    };

    let mut discarded = 0;

    for path in target_paths {
        if let Some(entry) = index.get(&path) {
            let full = repo.work_dir.join(&path);
            let blob_obj = repo.objects.read(&entry.oid)?;
            if let Some(p) = full.parent() {
                let _ = fs::create_dir_all(p);
            }
            fs::write(&full, &blob_obj.content).map_err(|e| e.to_string())?;
            discarded += 1;
        }
    }

    println!("\x1b[1;32m✓\x1b[0m Discarded modifications in \x1b[1;36m{}\x1b[0m file(s).", discarded);
    Ok(())
}
