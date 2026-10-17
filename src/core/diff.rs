use std::fs;
use similar::{ChangeTag, TextDiff};
use super::hash::Oid;
use super::repository::Repository;

pub struct DiffEngine;

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub old_oid: Option<Oid>,
    pub new_oid: Option<Oid>,
    pub status: &'static str,
    pub patch: String,
    pub additions: usize,
    pub deletions: usize,
}

impl DiffEngine {
    pub fn diff_text(
        old_name: &str,
        new_name: &str,
        old_content: &str,
        new_content: &str,
        color: bool,
    ) -> (String, usize, usize) {
        let diff = TextDiff::from_lines(old_content, new_content);
        let mut patch = String::new();
        let mut additions = 0;
        let mut deletions = 0;

        let reset = if color { "\x1b[0m" } else { "" };
        let bold = if color { "\x1b[1m" } else { "" };
        let cyan = if color { "\x1b[36m" } else { "" };
        let green = if color { "\x1b[32m" } else { "" };
        let red = if color { "\x1b[31m" } else { "" };

        patch.push_str(&format!("{}diff --tachyon a/{} b/{}{}\n", bold, old_name, new_name, reset));
        patch.push_str(&format!("--- a/{}\n", old_name));
        patch.push_str(&format!("+++ b/{}\n", new_name));

        for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
            patch.push_str(&format!("{}{}{}{}\n", cyan, hunk.header(), reset, ""));
            for change in hunk.iter_changes() {
                match change.tag() {
                    ChangeTag::Equal => {
                        patch.push_str(&format!(" {}", change.value()));
                    }
                    ChangeTag::Delete => {
                        deletions += 1;
                        patch.push_str(&format!("{}-{}", red, change.value()));
                        if color {
                            patch.push_str(reset);
                        }
                    }
                    ChangeTag::Insert => {
                        additions += 1;
                        patch.push_str(&format!("{}+{}", green, change.value()));
                        if color {
                            patch.push_str(reset);
                        }
                    }
                }
            }
        }

        (patch, additions, deletions)
    }

    pub fn diff_worktree_file(
        repo: &Repository,
        path: &str,
        color: bool,
    ) -> Result<Option<FileDiff>, String> {
        let index = repo.read_index().unwrap_or_default();
        let entry = index.get(path);
        let file_path = repo.work_dir.join(path);

        let old_content = match entry {
            Some(e) => {
                let obj = repo.objects.read(&e.oid)?;
                String::from_utf8_lossy(&obj.content).to_string()
            }
            None => String::new(),
        };

        let (new_content, status) = if file_path.exists() {
            let bytes = fs::read(&file_path).map_err(|e| e.to_string())?;
            (String::from_utf8_lossy(&bytes).to_string(), if entry.is_some() { "modified" } else { "untracked" })
        } else {
            (String::new(), "deleted")
        };

        if old_content == new_content {
            return Ok(None);
        }

        let (patch, additions, deletions) = Self::diff_text(path, path, &old_content, &new_content, color);
        Ok(Some(FileDiff {
            path: path.to_string(),
            old_oid: entry.map(|e| e.oid),
            new_oid: None,
            status,
            patch,
            additions,
            deletions,
        }))
    }

    pub fn diff_staged_file(
        repo: &Repository,
        path: &str,
        head_oid: Option<Oid>,
        color: bool,
    ) -> Result<Option<FileDiff>, String> {
        let index = repo.read_index().unwrap_or_default();
        let entry = index.get(path);

        let old_content = if let Some(oid) = head_oid {
            if let Ok(obj) = repo.objects.read(&oid) {
                String::from_utf8_lossy(&obj.content).to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let (new_content, status) = match entry {
            Some(e) => {
                let obj = repo.objects.read(&e.oid)?;
                (String::from_utf8_lossy(&obj.content).to_string(), if head_oid.is_some() { "modified" } else { "new file" })
            }
            None => (String::new(), "deleted"),
        };

        if old_content == new_content {
            return Ok(None);
        }

        let (patch, additions, deletions) = Self::diff_text(path, path, &old_content, &new_content, color);
        Ok(Some(FileDiff {
            path: path.to_string(),
            old_oid: head_oid,
            new_oid: entry.map(|e| e.oid),
            status,
            patch,
            additions,
            deletions,
        }))
    }
}
