use std::process::Command;
use crate::core::repository::Repository;

pub fn execute(remote: Option<String>, all: bool, prune: bool) -> Result<(), String> {
    let repo = Repository::find_root()?;
    let remote_name = remote.unwrap_or_else(|| "origin".to_string());

    if all {
        println!("\x1b[1;36m⚡ Fetching updates from all remotes...\x1b[0m");
    } else {
        println!("\x1b[1;36m⚡ Fetching updates from '{}'...\x1b[0m", remote_name);
    }

    let mut cmd = Command::new("git");
    cmd.current_dir(&repo.work_dir)
        .arg("fetch");

    if all {
        cmd.arg("--all");
    } else {
        cmd.arg(&remote_name);
    }

    if prune {
        cmd.arg("--prune");
    }

    let status = cmd.status().map_err(|e| format!("Failed to execute fetch: {}", e))?;
    if !status.success() {
        return Err(format!("Fetch failed with exit code {:?}", status.code()));
    }

    println!("\x1b[1;32m✓\x1b[0m Remote objects and tracking branches up to date.");
    Ok(())
}
