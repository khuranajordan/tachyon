use std::io::IsTerminal;
use crate::core::checkout::CheckoutEngine;
use crate::core::repository::Repository;
use crate::interactive::branch_picker::BranchPicker;

pub fn execute(target: Option<String>, create_branch: Option<String>, force: bool) -> Result<(), String> {
    let repo = Repository::find_root()?;

    if let Some(new_branch) = create_branch {
        let head_oid = repo.head_commit()?
            .ok_or_else(|| "Cannot create branch: repository has no commits yet.".to_string())?;
        repo.create_branch(&new_branch, &head_oid)?;
        repo.set_head_branch(&new_branch)?;
        println!("\x1b[1;32m✓\x1b[0m Switched to a new branch \x1b[1;32m'{}'\x1b[0m", new_branch);
        return Ok(());
    }

    let branch_to_switch = match target {
        Some(ref t) if t == "-" || t == "@{-1}" => {
            resolve_previous_branch(&repo)?
        }
        Some(t) => t,
        None => {
            if std::io::stdout().is_terminal() {
                let branches = repo.list_branches()?;
                match BranchPicker::select_branch(&branches)? {
                    Some(b) => b,
                    None => {
                        println!("\x1b[90mBranch switch cancelled.\x1b[0m");
                        return Ok(());
                    }
                }
            } else {
                return Err("No branch specified. Use 'tachyon jump <branch>' or 'tachyon jump -b <new-branch>'".to_string());
            }
        }
    };

    // Check if target is a branch name
    if repo.read_branch(&branch_to_switch).is_ok() {
        CheckoutEngine::switch_branch(&repo, &branch_to_switch, force)?;
        println!("\x1b[1;32m✓\x1b[0m Switched to branch \x1b[1;32m'{}'\x1b[0m", branch_to_switch);
        return Ok(());
    }

    // Otherwise check if target is a commit hash
    if let Ok(oid) = repo.resolve_oid(&branch_to_switch) {
        CheckoutEngine::checkout_commit(&repo, &oid, None, force)?;
        println!("\x1b[1;33m● Note: switching to detached HEAD at '{}'\x1b[0m", oid.short());
        return Ok(());
    }

    Err(format!("error: pathspec or branch '{}' did not match any file(s) known to tachyon", branch_to_switch))
}

fn resolve_previous_branch(repo: &Repository) -> Result<String, String> {
    let current = repo.current_branch_name();

    // 1. Scan .git/logs/HEAD backwards
    let log_path = repo.git_dir.join("logs").join("HEAD");
    if log_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&log_path) {
            for line in content.lines().rev() {
                if let Some(pos) = line.find("\tcheckout: moving from ") {
                    let rest = &line[pos + "\tcheckout: moving from ".len()..];
                    if let Some(to_pos) = rest.find(" to ") {
                        let from_branch = rest[..to_pos].trim();
                        let from_branch = from_branch.strip_prefix("refs/heads/").unwrap_or(from_branch);
                        if from_branch != current && !from_branch.is_empty() {
                            return Ok(from_branch.to_string());
                        }
                    }
                } else if let Some(pos) = line.find("\trebase (finish): returning to refs/heads/") {
                    let from_branch = line[pos + "\trebase (finish): returning to refs/heads/".len()..].trim();
                    if from_branch != current && !from_branch.is_empty() {
                        return Ok(from_branch.to_string());
                    }
                }
            }
        }
    }

    // 2. Check .git/PREV_BRANCH
    let prev_branch_file = repo.git_dir.join("PREV_BRANCH");
    if prev_branch_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&prev_branch_file) {
            let prev = content.trim();
            if prev != current && !prev.is_empty() {
                return Ok(prev.to_string());
            }
        }
    }

    // 3. Fallback to git rev-parse --symbolic-full-name @{-1}
    if let Ok(output) = std::process::Command::new("git")
        .current_dir(&repo.work_dir)
        .args(&["rev-parse", "--symbolic-full-name", "@{-1}"])
        .output()
    {
        if output.status.success() {
            let full_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let branch = full_name.strip_prefix("refs/heads/").unwrap_or(&full_name);
            if !branch.is_empty() && branch != current {
                return Ok(branch.to_string());
            }
        }
    }

    Err("error: no previous branch found in reflog".to_string())
}
