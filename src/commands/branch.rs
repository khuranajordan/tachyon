use crate::core::repository::Repository;

pub fn execute(
    name: Option<String>,
    delete: Option<String>,
    rename: Option<(String, String)>,
    create_at: Option<String>,
) -> Result<(), String> {
    let repo = Repository::find_root()?;

    if let Some(target) = delete {
        repo.delete_branch(&target)?;
        println!("\x1b[1;32m✓\x1b[0m Deleted branch \x1b[33m{}\x1b[0m.", target);
        return Ok(());
    }

    if let Some((old_name, new_name)) = rename {
        let oid = repo.read_branch(&old_name)?;
        repo.create_branch(&new_name, &oid)?;
        repo.delete_branch(&old_name)?;
        if repo.current_branch_name() == old_name {
            repo.set_head_branch(&new_name)?;
        }
        println!("\x1b[1;32m✓\x1b[0m Renamed branch \x1b[33m{}\x1b[0m to \x1b[32m{}\x1b[0m.", old_name, new_name);
        return Ok(());
    }

    if let Some(new_branch) = name {
        let head_oid = match create_at {
            Some(ref target) => crate::core::hash::Oid::from_hex(target)?,
            None => repo.head_commit()?.ok_or_else(|| "Cannot create branch: no commits in repository".to_string())?,
        };

        repo.create_branch(&new_branch, &head_oid)?;
        println!("\x1b[1;32m✓\x1b[0m Created branch \x1b[1;32m{}\x1b[0m at {}.", new_branch, head_oid.short());
        return Ok(());
    }

    // List branches
    let branches = repo.list_branches()?;
    if branches.is_empty() {
        println!("No branches found.");
        return Ok(());
    }

    println!("\x1b[1;36mBranches:\x1b[0m");
    for (b_name, oid, is_current) in branches {
        if is_current {
            println!("  \x1b[1;32m* {:20}\x1b[0m \x1b[90m{}\x1b[0m", b_name, oid.short());
        } else {
            println!("    {:20} \x1b[90m{}\x1b[0m", b_name, oid.short());
        }
    }

    Ok(())
}
