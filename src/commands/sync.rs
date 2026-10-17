use std::process::Command;
use crate::core::repository::Repository;

pub fn execute(remote: Option<String>, branch: Option<String>, rebase: bool) -> Result<(), String> {
    let repo = Repository::find_root()?;
    let current_branch = repo.current_branch_name();
    let remote_name = remote.unwrap_or_else(|| "origin".to_string());
    let branch_name = branch.unwrap_or(current_branch);

    println!("\x1b[1;36m⚡ Syncing with {}/{}...\x1b[0m", remote_name, branch_name);

    let mut cmd = Command::new("git");
    cmd.current_dir(&repo.work_dir)
        .arg("pull");

    if rebase {
        cmd.arg("--rebase");
    }

    cmd.arg(&remote_name).arg(&branch_name);

    let status = cmd.status().map_err(|e| format!("Failed to execute sync: {}", e))?;
    if !status.success() {
        return Err(format!("Sync failed with exit code {:?}", status.code()));
    }

    println!("\x1b[1;32m✓\x1b[0m Repository successfully synchronized with {}/{}", remote_name, branch_name);
    Ok(())
}
