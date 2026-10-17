use std::process::Command;
use crate::core::repository::Repository;

pub fn execute(remote: Option<String>, branch: Option<String>, set_upstream: bool, force: bool) -> Result<(), String> {
    let repo = Repository::find_root()?;
    let current_branch = repo.current_branch_name();
    let remote_name = remote.unwrap_or_else(|| "origin".to_string());
    let branch_name = branch.unwrap_or(current_branch);

    println!("\x1b[1;36m⚡ Publishing to {}/{}...\x1b[0m", remote_name, branch_name);

    let mut cmd = Command::new("git");
    cmd.current_dir(&repo.work_dir)
        .arg("push");

    if set_upstream {
        cmd.arg("-u");
    }
    if force {
        cmd.arg("--force-with-lease");
    }

    cmd.arg(&remote_name).arg(&branch_name);

    let status = cmd.status().map_err(|e| format!("Failed to execute publish: {}", e))?;
    if !status.success() {
        return Err(format!("Publish failed with exit code {:?}", status.code()));
    }

    println!("\x1b[1;32m✓\x1b[0m Successfully published to {}/{}", remote_name, branch_name);
    Ok(())
}
