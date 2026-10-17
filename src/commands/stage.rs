use std::fs;
use rayon::prelude::*;
use crate::core::hash::Oid;
use crate::core::ignore::GitIgnore;
use crate::core::index::IndexEntry;
use crate::core::repository::Repository;
use crate::core::status::StatusScanner;
use crate::interactive::stage_selector::{SelectableFile, StageSelector};

pub fn execute(paths: Vec<String>, all: bool, interactive: bool) -> Result<(), String> {
    let repo = Repository::find_root()?;
    let status = StatusScanner::scan(&repo)?;

    let target_paths: Vec<String> = if all || (!paths.is_empty() && paths.contains(&".".to_string())) {
        status.unstaged_modified.into_iter()
            .chain(status.unstaged_deleted.into_iter())
            .chain(status.untracked.into_iter())
            .collect()
    } else if !paths.is_empty() {
        paths
    } else if interactive || atty_check() {
        // Prepare list for interactive stage selector
        let mut items = Vec::new();
        for p in &status.unstaged_modified {
            items.push(SelectableFile { path: p.clone(), status: "modified", selected: true });
        }
        for p in &status.unstaged_deleted {
            items.push(SelectableFile { path: p.clone(), status: "deleted", selected: true });
        }
        for p in &status.untracked {
            items.push(SelectableFile { path: p.clone(), status: "untracked", selected: true });
        }

        if items.is_empty() {
            println!("\x1b[32mNothing to stage. Working tree clean.\x1b[0m");
            return Ok(());
        }

        StageSelector::select_files_interactively(items)?
    } else if all {
        status.unstaged_modified.into_iter()
            .chain(status.unstaged_deleted.into_iter())
            .chain(status.untracked.into_iter())
            .collect()
    } else {
        return Err("No files specified. Use 'tachyon stage <files>...' or 'tachyon stage -a' or run interactively in a terminal.".to_string());
    };

    if target_paths.is_empty() {
        println!("\x1b[90mStaging cancelled or no files selected.\x1b[0m");
        return Ok(());
    }

    let mut index = repo.read_index().unwrap_or_default();
    let gitignore = GitIgnore::new(&repo.work_dir);

    // Read and hash files in parallel with Rayon
    let updates: Vec<(String, Option<(IndexEntry, Vec<u8>)>)> = target_paths
        .par_iter()
        .map(|path_str| {
            let full_path = repo.work_dir.join(path_str);
            if !full_path.exists() {
                // File deleted
                return (path_str.clone(), None);
            }

            if gitignore.is_ignored(path_str, full_path.is_dir()) {
                return (path_str.clone(), None);
            }

            if let Ok(meta) = fs::metadata(&full_path) {
                if meta.is_file() {
                    if let Ok(bytes) = fs::read(&full_path) {
                        let oid = Oid::hash_object("blob", &bytes);
                        let (mtime_sec, mtime_nsec) = meta
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| (d.as_secs() as u32, d.subsec_nanos()))
                            .unwrap_or((0, 0));

                        let mode = 0o100644; // default regular file mode

                        let entry = IndexEntry {
                            ctime_sec: mtime_sec,
                            ctime_nsec: mtime_nsec,
                            mtime_sec,
                            mtime_nsec,
                            dev: 0,
                            ino: 0,
                            mode,
                            uid: 0,
                            gid: 0,
                            file_size: meta.len() as u32,
                            oid,
                            flags: (path_str.len().min(0xFFF)) as u16,
                            path: path_str.clone(),
                        };

                        return (path_str.clone(), Some((entry, bytes)));
                    }
                }
            }

            (path_str.clone(), None)
        })
        .collect();

    let mut staged_count = 0;
    for (path, opt) in updates {
        match opt {
            Some((entry, bytes)) => {
                repo.objects.write_blob(&bytes)?;
                index.add(entry);
                staged_count += 1;
            }
            None => {
                let full = repo.work_dir.join(&path);
                if !full.exists() {
                    index.remove(&path);
                    staged_count += 1;
                }
            }
        }
    }

    repo.write_index(&index)?;
    println!("\x1b[1;32m✓\x1b[0m Staged \x1b[1;36m{}\x1b[0m file(s) for next save.", staged_count);
    Ok(())
}

fn atty_check() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}
