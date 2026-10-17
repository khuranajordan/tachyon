use std::collections::HashSet;
use chrono::{Local, TimeZone};
use crate::core::repository::Repository;
use crate::interactive::history_browser::{CommitSummary, HistoryBrowser};

pub fn execute(limit: Option<usize>, oneline: bool, interactive: bool) -> Result<(), String> {
    let repo = Repository::find_root()?;
    let head_oid = match repo.head_commit()? {
        Some(oid) => oid,
        None => {
            println!("No commits yet on branch {}", repo.current_branch_name());
            return Ok(());
        }
    };

    // Collect commit history
    let mut commits = Vec::new();
    let mut queue = vec![head_oid];
    let mut visited = HashSet::new();
    let max = limit.unwrap_or(usize::MAX);

    while let Some(curr_oid) = queue.pop() {
        if !visited.insert(curr_oid) {
            continue;
        }

        let commit = repo.objects.parse_commit(&curr_oid)?;
        let title = commit.message.lines().next().unwrap_or("").to_string();
        let author_name = extract_author_name(&commit.author);
        let date_str = extract_date_str(&commit.author);

        commits.push((curr_oid, commit.clone(), title, author_name, date_str));

        if commits.len() >= max {
            break;
        }

        for parent in commit.parents {
            queue.push(parent);
        }
    }

    if interactive {
        let summaries: Vec<CommitSummary> = commits
            .into_iter()
            .map(|(oid, _, title, author, date)| CommitSummary {
                oid,
                author,
                date,
                title,
            })
            .collect();

        return HistoryBrowser::browse(&repo, summaries);
    }

    // Standard CLI output
    for (idx, (oid, commit, title, author, date)) in commits.iter().enumerate() {
        if oneline {
            println!(
                "\x1b[35m*\x1b[0m \x1b[33m{}\x1b[0m \x1b[1m{}\x1b[0m \x1b[90m({})\x1b[0m",
                oid.short(), title, author
            );
        } else {
            let branch_decorations = if idx == 0 {
                format!(" \x1b[32m(HEAD -> {})\x1b[0m", repo.current_branch_name())
            } else {
                String::new()
            };

            println!(
                "\x1b[33mcommit {}{}\x1b[0m",
                oid.to_hex(), branch_decorations
            );
            println!("Author: {}", commit.author);
            println!("Date:   {}", date);
            println!();
            for line in commit.message.lines() {
                println!("    {}", line);
            }
            println!();
        }
    }

    Ok(())
}

fn extract_author_name(author_str: &str) -> String {
    if let Some(idx) = author_str.find('<') {
        author_str[..idx].trim().to_string()
    } else {
        author_str.split_whitespace().next().unwrap_or("Unknown").to_string()
    }
}

fn extract_date_str(author_str: &str) -> String {
    let parts: Vec<&str> = author_str.split_whitespace().collect();
    if parts.len() >= 2 {
        if let Ok(ts) = parts[parts.len() - 2].parse::<i64>() {
            if let Some(dt) = Local.timestamp_opt(ts, 0).single() {
                return dt.format("%a %b %e %T %Y").to_string();
            }
        }
    }
    "".to_string()
}
