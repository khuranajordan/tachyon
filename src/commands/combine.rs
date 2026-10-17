use crate::core::merge::{MergeEngine, MergeResult};
use crate::core::repository::Repository;

pub fn execute(target_branch: String) -> Result<(), String> {
    let repo = Repository::find_root()?;

    match MergeEngine::merge(&repo, &target_branch)? {
        MergeResult::FastForward(oid) => {
            println!(
                "\x1b[1;32m✓ Fast-forward merge:\x1b[0m updated to commit \x1b[33m{}\x1b[0m",
                oid.short()
            );
        }
        MergeResult::AlreadyUpToDate => {
            println!("\x1b[32mAlready up to date.\x1b[0m");
        }
        MergeResult::Merged(oid) => {
            println!(
                "\x1b[1;32m✓ Successfully combined branch '{}':\x1b[0m created merge commit \x1b[33m{}\x1b[0m",
                target_branch, oid.short()
            );
        }
        MergeResult::Conflict(conflicts) => {
            eprintln!("\x1b[1;31mCONFLICT:\x1b[0m Automatic merge failed in the following file(s):");
            for f in conflicts {
                eprintln!("  \x1b[31m! {}\x1b[0m", f);
            }
            eprintln!("\x1b[33mFix conflicts and then run 'tachyon save' to conclude merge.\x1b[0m");
        }
    }

    Ok(())
}
