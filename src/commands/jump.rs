use std::io::IsTerminal;
use crate::core::checkout::CheckoutEngine;
use crate::core::hash::Oid;
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
    if let Ok(oid) = Oid::from_hex(&branch_to_switch) {
        CheckoutEngine::checkout_commit(&repo, &oid, None, force)?;
        println!("\x1b[1;33m● Note: switching to detached HEAD at '{}'\x1b[0m", oid.short());
        return Ok(());
    }

    Err(format!("error: pathspec or branch '{}' did not match any file(s) known to tachyon", branch_to_switch))
}
