use std::fs::{self, OpenOptions};
use std::io::Write;
use chrono::Local;
use super::hash::Oid;
use super::object::CommitObject;
use super::repository::{HeadState, Repository};
use super::tree::TreeBuilder;

pub struct Committer;

impl Committer {
    pub fn commit(
        repo: &Repository,
        message: &str,
        custom_author: Option<&str>,
        extra_parents: &[Oid],
    ) -> Result<Oid, String> {
        let index = repo.read_index()?;
        if index.entries.is_empty() {
            return Err("nothing to commit (index is empty)".to_string());
        }

        // Build tree from index
        let tree_oid = TreeBuilder::build_from_index(&index, &repo.objects)?;

        // Determine parent(s)
        let head_commit = repo.head_commit()?;
        let mut parents = Vec::new();
        if let Some(parent) = head_commit {
            // Check if tree has actually changed compared to parent commit's tree
            if let Ok(parent_commit) = repo.objects.parse_commit(&parent) {
                if parent_commit.tree == tree_oid && extra_parents.is_empty() {
                    return Err("nothing to commit, working tree clean".to_string());
                }
            }
            parents.push(parent);
        }
        parents.extend_from_slice(extra_parents);

        // Author and committer
        let (default_name, default_email) = repo.author_identity();
        let author_str = if let Some(author) = custom_author {
            author.to_string()
        } else {
            let now = Local::now();
            let timestamp = now.timestamp();
            let offset = now.offset().local_minus_utc();
            let offset_sign = if offset >= 0 { '+' } else { '-' };
            let offset_mins = (offset.abs() / 60) % 60;
            let offset_hours = offset.abs() / 3600;
            format!(
                "{} <{}> {} {}{:02}{:02}",
                default_name, default_email, timestamp, offset_sign, offset_hours, offset_mins
            )
        };

        let now = Local::now();
        let timestamp = now.timestamp();
        let offset = now.offset().local_minus_utc();
        let offset_sign = if offset >= 0 { '+' } else { '-' };
        let offset_mins = (offset.abs() / 60) % 60;
        let offset_hours = offset.abs() / 3600;
        let committer_str = format!(
            "{} <{}> {} {}{:02}{:02}",
            default_name, default_email, timestamp, offset_sign, offset_hours, offset_mins
        );

        let commit_obj = CommitObject {
            tree: tree_oid,
            parents: parents.clone(),
            author: author_str.clone(),
            committer: committer_str.clone(),
            message: message.trim().to_string(),
        };

        let commit_oid = repo.objects.write_commit(&commit_obj)?;

        // Update branch ref or HEAD
        let head_state = repo.read_head()?;
        match head_state {
            HeadState::Branch { name, .. } => {
                repo.write_branch(&name, &commit_oid)?;
                Self::append_reflog(
                    repo,
                    &format!("refs/heads/{}", name),
                    head_commit.unwrap_or(Oid::ZERO),
                    commit_oid,
                    &committer_str,
                    &format!("commit: {}", message.lines().next().unwrap_or("")),
                );
            }
            HeadState::Detached(..) => {
                repo.set_head_detached(&commit_oid)?;
            }
        }

        Self::append_reflog(
            repo,
            "HEAD",
            head_commit.unwrap_or(Oid::ZERO),
            commit_oid,
            &committer_str,
            &format!("commit: {}", message.lines().next().unwrap_or("")),
        );

        Ok(commit_oid)
    }

    fn append_reflog(
        repo: &Repository,
        ref_name: &str,
        old_oid: Oid,
        new_oid: Oid,
        committer: &str,
        msg: &str,
    ) {
        let log_path = repo.git_dir.join("logs").join(ref_name);
        if let Some(parent) = log_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let line = format!("{} {} {}\t{}\n", old_oid.to_hex(), new_oid.to_hex(), committer, msg);
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
            let _ = file.write_all(line.as_bytes());
        }
    }
}
