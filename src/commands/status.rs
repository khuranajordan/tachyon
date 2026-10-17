use crate::core::repository::Repository;
use crate::core::status::StatusScanner;

pub fn execute() -> Result<(), String> {
    let repo = Repository::find_root()?;
    let status = StatusScanner::scan(&repo)?;

    println!("\x1b[1;34m● On branch \x1b[1;32m{}\x1b[0m", status.branch);
    if let Some(head) = status.head_oid {
        println!("\x1b[90m  Head commit: {}\x1b[0m", head.short());
    } else {
        println!("\x1b[90m  No commits yet\x1b[0m");
    }
    println!();

    if status.is_clean() {
        println!("\x1b[1;32m✓ Working tree clean. Nothing to save.\x1b[0m");
        return Ok(());
    }

    if status.has_staged() {
        println!("\x1b[1;32mChanges staged for next save (commit):\x1b[0m");
        println!("\x1b[90m  (use \"tachyon unstage <file>...\" to unstage)\x1b[0m");
        for f in &status.staged_new {
            println!("  \x1b[32m+ new file:   {}\x1b[0m", f);
        }
        for f in &status.staged_modified {
            println!("  \x1b[32m~ modified:   {}\x1b[0m", f);
        }
        for f in &status.staged_deleted {
            println!("  \x1b[31m- deleted:    {}\x1b[0m", f);
        }
        println!();
    }

    if !status.unstaged_modified.is_empty() || !status.unstaged_deleted.is_empty() {
        println!("\x1b[1;33mChanges not staged for save:\x1b[0m");
        println!("\x1b[90m  (use \"tachyon stage <file>...\" to track)\x1b[0m");
        println!("\x1b[90m  (use \"tachyon discard <file>...\" to discard working tree changes)\x1b[0m");
        for f in &status.unstaged_modified {
            println!("  \x1b[33m~ modified:   {}\x1b[0m", f);
        }
        for f in &status.unstaged_deleted {
            println!("  \x1b[31m- deleted:    {}\x1b[0m", f);
        }
        println!();
    }

    if !status.untracked.is_empty() {
        println!("\x1b[1;35mUntracked files:\x1b[0m");
        println!("\x1b[90m  (use \"tachyon stage <file>...\" to include in what will be saved)\x1b[0m");
        for f in &status.untracked {
            println!("  \x1b[35m? {}\x1b[0m", f);
        }
        println!();
    }

    Ok(())
}
