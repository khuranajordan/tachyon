use crate::core::repository::Repository;

pub fn execute(name: Option<String>, target_commit: Option<String>) -> Result<(), String> {
    let repo = Repository::find_root()?;

    if let Some(tag_name) = name {
        let oid = match target_commit {
            Some(ref t) => repo.resolve_oid(t)?,
            None => repo.head_commit()?.ok_or_else(|| "No commits to tag".to_string())?,
        };

        repo.create_tag(&tag_name, &oid)?;
        println!("\x1b[1;32m✓\x1b[0m Created milestone \x1b[1;33m'{}'\x1b[0m at {}.", tag_name, oid.short());
        return Ok(());
    }

    let tags = repo.list_tags()?;
    if tags.is_empty() {
        println!("No milestones (tags) found.");
        return Ok(());
    }

    println!("\x1b[1;36mMilestones (Tags):\x1b[0m");
    for (t_name, oid) in tags {
        println!("  \x1b[33m{:20}\x1b[0m \x1b[90m{}\x1b[0m", t_name, oid.short());
    }

    Ok(())
}
