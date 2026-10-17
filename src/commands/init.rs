use std::env;
use std::path::PathBuf;
use crate::core::repository::Repository;

pub fn execute(dir: Option<String>, initial_branch: Option<String>) -> Result<(), String> {
    let target_dir = match dir {
        Some(d) => PathBuf::from(d),
        None => env::current_dir().map_err(|e| e.to_string())?,
    };

    let branch = initial_branch.unwrap_or_else(|| "main".to_string());
    let repo = Repository::init(&target_dir, &branch)?;

    println!(
        "\x1b[1;32m✓\x1b[0m Initialized empty Tachyon/Git repository in \x1b[1;36m{:?}\x1b[0m (branch: \x1b[33m{}\x1b[0m)",
        repo.git_dir, branch
    );
    Ok(())
}
