use std::io::IsTerminal;
use crate::core::commit::Committer;
use crate::core::repository::Repository;
use crate::core::status::StatusScanner;
use crate::interactive::commit_wizard::CommitWizard;

pub fn execute(message: Option<String>, all: bool, author: Option<String>) -> Result<(), String> {
    let repo = Repository::find_root()?;

    // If --all, automatically stage modified and deleted files
    if all {
        super::stage::execute(vec![], true, false)?;
    }

    let status = StatusScanner::scan(&repo)?;
    if !status.has_staged() {
        return Err("No changes staged to save. Use 'tachyon stage <files>...' or 'tachyon save -a' to stage automatically.".to_string());
    }

    let commit_message = match message {
        Some(m) if !m.trim().is_empty() => m,
        _ => {
            if std::io::stdout().is_terminal() {
                match CommitWizard::prompt_commit_message()? {
                    Some(m) => m,
                    None => {
                        println!("\x1b[90mSave cancelled.\x1b[0m");
                        return Ok(());
                    }
                }
            } else {
                return Err("Aborting save due to empty commit message. Use -m \"<message>\" in non-interactive environments.".to_string());
            }
        }
    };

    let commit_oid = Committer::commit(&repo, &commit_message, author.as_deref(), &[])?;
    let branch = repo.current_branch_name();

    let title = commit_message.lines().next().unwrap_or("");
    let changed_count = status.staged_new.len() + status.staged_modified.len() + status.staged_deleted.len();

    println!(
        "\x1b[1;32m[{} {}]\x1b[0m {}",
        branch,
        commit_oid.short(),
        title
    );
    println!(
        " \x1b[90m{} file(s) changed (+{} new, ~{} modified, -{} deleted)\x1b[0m",
        changed_count,
        status.staged_new.len(),
        status.staged_modified.len(),
        status.staged_deleted.len()
    );

    Ok(())
}
