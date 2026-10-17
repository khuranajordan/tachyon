use std::collections::HashSet;
use super::hash::Oid;
use super::repository::Repository;
use super::tree::TreeBuilder;

#[derive(Debug, Clone)]
pub struct BlameLine {
    pub line_num: usize,
    pub commit_oid: Oid,
    pub author: String,
    pub content: String,
}

pub struct BlameEngine;

impl BlameEngine {
    pub fn blame_file(repo: &Repository, path: &str) -> Result<Vec<BlameLine>, String> {
        let head_commit = repo.head_commit()?
            .ok_or_else(|| "No commits in repository".to_string())?;

        // Read file content at HEAD
        let head_obj = repo.objects.parse_commit(&head_commit)?;
        let head_tree = TreeBuilder::read_tree_recursive(&repo.objects, &head_obj.tree, "")?;
        let blob_oid = head_tree.get(path)
            .map(|(_, oid)| *oid)
            .ok_or_else(|| format!("File '{}' not found at HEAD", path))?;

        let blob = repo.objects.read(&blob_oid)?;
        let text = String::from_utf8_lossy(&blob.content);
        let lines: Vec<&str> = text.lines().collect();

        // For each line, initialize with head commit
        let mut result: Vec<BlameLine> = lines
            .iter()
            .enumerate()
            .map(|(i, &l)| BlameLine {
                line_num: i + 1,
                commit_oid: head_commit,
                author: Self::extract_author_name(&head_obj.author),
                content: l.to_string(),
            })
            .collect();

        // Walk back parents to see where lines originated
        let mut curr_commit_oid = head_commit;
        let mut visited = HashSet::new();

        while visited.insert(curr_commit_oid) {
            let curr_commit = match repo.objects.parse_commit(&curr_commit_oid) {
                Ok(c) => c,
                Err(_) => break,
            };

            if curr_commit.parents.is_empty() {
                break;
            }

            let parent_oid = curr_commit.parents[0];
            let parent_commit = match repo.objects.parse_commit(&parent_oid) {
                Ok(c) => c,
                Err(_) => break,
            };

            let parent_tree = match TreeBuilder::read_tree_recursive(&repo.objects, &parent_commit.tree, "") {
                Ok(t) => t,
                Err(_) => break,
            };

            if let Some((_, parent_blob_oid)) = parent_tree.get(path) {
                if let Ok(parent_blob) = repo.objects.read(parent_blob_oid) {
                    let parent_text = String::from_utf8_lossy(&parent_blob.content);
                    let parent_lines: Vec<&str> = parent_text.lines().collect();

                    // If a line in result matches a line in parent, attribute to parent
                    for bl in &mut result {
                        if parent_lines.contains(&bl.content.as_str()) {
                            bl.commit_oid = parent_oid;
                            bl.author = Self::extract_author_name(&parent_commit.author);
                        }
                    }
                }
            } else {
                // File didn't exist in parent -> current commit introduced all remaining lines
                break;
            }

            curr_commit_oid = parent_oid;
        }

        Ok(result)
    }

    fn extract_author_name(author_str: &str) -> String {
        if let Some(idx) = author_str.find('<') {
            author_str[..idx].trim().to_string()
        } else {
            author_str.split_whitespace().next().unwrap_or("Unknown").to_string()
        }
    }
}
