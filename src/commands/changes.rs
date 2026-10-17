use std::collections::BTreeMap;
use crate::core::diff::DiffEngine;
use crate::core::hash::Oid;
use crate::core::repository::Repository;
use crate::core::status::StatusScanner;
use crate::core::tree::TreeBuilder;

pub fn execute(staged: bool, path: Option<String>, commits: Vec<String>) -> Result<(), String> {
    let repo = Repository::find_root()?;

    if commits.len() == 2 {
        let oid1 = repo.resolve_oid(&commits[0])?;
        let oid2 = repo.resolve_oid(&commits[1])?;
        diff_between_commits(&repo, &oid1, &oid2, path.as_deref())?;
        return Ok(());
    }

    let status = StatusScanner::scan(&repo)?;
    let mut total_additions = 0;
    let mut total_deletions = 0;
    let mut files_diffed = 0;

    if staged {
        // Compare Index vs HEAD
        let head_files: BTreeMap<String, (u32, Oid)> = if let Some(head) = status.head_oid {
            if let Ok(c) = repo.objects.parse_commit(&head) {
                TreeBuilder::read_tree_recursive(&repo.objects, &c.tree, "").unwrap_or_default()
            } else {
                BTreeMap::new()
            }
        } else {
            BTreeMap::new()
        };

        let target_files: Vec<String> = if let Some(p) = path {
            vec![p]
        } else {
            status.staged_new.iter()
                .chain(status.staged_modified.iter())
                .chain(status.staged_deleted.iter())
                .cloned()
                .collect()
        };

        for p in target_files {
            let head_oid = head_files.get(&p).map(|(_, oid)| *oid);
            if let Ok(Some(diff)) = DiffEngine::diff_staged_file(&repo, &p, head_oid, true) {
                print!("{}", diff.patch);
                total_additions += diff.additions;
                total_deletions += diff.deletions;
                files_diffed += 1;
            }
        }
    } else {
        // Compare Working Tree vs Index
        let target_files: Vec<String> = if let Some(p) = path {
            vec![p]
        } else {
            status.unstaged_modified.iter()
                .chain(status.unstaged_deleted.iter())
                .cloned()
                .collect()
        };

        for p in target_files {
            if let Ok(Some(diff)) = DiffEngine::diff_worktree_file(&repo, &p, true) {
                print!("{}", diff.patch);
                total_additions += diff.additions;
                total_deletions += diff.deletions;
                files_diffed += 1;
            }
        }
    }

    if files_diffed > 0 {
        println!(
            "\x1b[90mSummary: {} file(s) changed, \x1b[32m+{} insertions\x1b[90m, \x1b[31m-{} deletions\x1b[0m",
            files_diffed, total_additions, total_deletions
        );
    } else {
        println!("\x1b[90mNo changes to display.\x1b[0m");
    }

    Ok(())
}

fn diff_between_commits(
    repo: &Repository,
    oid1: &Oid,
    oid2: &Oid,
    path_filter: Option<&str>,
) -> Result<(), String> {
    let commit1 = repo.objects.parse_commit(oid1)?;
    let commit2 = repo.objects.parse_commit(oid2)?;

    let files1 = TreeBuilder::read_tree_recursive(&repo.objects, &commit1.tree, "")?;
    let files2 = TreeBuilder::read_tree_recursive(&repo.objects, &commit2.tree, "")?;

    let mut all_paths = std::collections::BTreeSet::new();
    for p in files1.keys() { all_paths.insert(p.clone()); }
    for p in files2.keys() { all_paths.insert(p.clone()); }

    for p in all_paths {
        if let Some(filter) = path_filter {
            if p != filter {
                continue;
            }
        }

        let blob1 = files1.get(&p).map(|(_, oid)| *oid);
        let blob2 = files2.get(&p).map(|(_, oid)| *oid);

        if blob1 == blob2 {
            continue;
        }

        let text1 = if let Some(oid) = blob1 {
            let obj = repo.objects.read(&oid)?;
            String::from_utf8_lossy(&obj.content).to_string()
        } else {
            String::new()
        };

        let text2 = if let Some(oid) = blob2 {
            let obj = repo.objects.read(&oid)?;
            String::from_utf8_lossy(&obj.content).to_string()
        } else {
            String::new()
        };

        let (patch, _, _) = DiffEngine::diff_text(&p, &p, &text1, &text2, true);
        print!("{}", patch);
    }

    Ok(())
}
