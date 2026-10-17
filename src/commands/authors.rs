use crate::core::blame::BlameEngine;
use crate::core::repository::Repository;

pub fn execute(file_path: String) -> Result<(), String> {
    let repo = Repository::find_root()?;
    let blame_lines = BlameEngine::blame_file(&repo, &file_path)?;

    println!("\x1b[1;36m=== Authors for: {} ===\x1b[0m", file_path);
    for line in blame_lines {
        println!(
            "\x1b[33m{}\x1b[0m (\x1b[36m{:12}\x1b[0m {:4}) {}",
            line.commit_oid.short(),
            line.author,
            line.line_num,
            line.content
        );
    }

    Ok(())
}
