use std::collections::BTreeMap;
use std::io::{stdout, Write};
use crossterm::{
    cursor,
    event::KeyCode,
    execute,
    terminal::{self, ClearType},
};
use super::tui::{read_key, TerminalGuard};
use crate::core::diff::DiffEngine;
use crate::core::hash::Oid;
use crate::core::repository::Repository;
use crate::core::tree::TreeBuilder;

pub struct CommitSummary {
    pub oid: Oid,
    pub author: String,
    pub date: String,
    pub title: String,
}

pub struct HistoryBrowser;

impl HistoryBrowser {
    pub fn browse(repo: &Repository, commits: Vec<CommitSummary>) -> Result<(), String> {
        if commits.is_empty() {
            println!("No commits yet in this repository.");
            return Ok(());
        }

        let _guard = TerminalGuard::enter_raw()?;
        let mut cursor_idx = 0;

        loop {
            execute!(stdout(), cursor::MoveTo(0, 0), terminal::Clear(ClearType::All))
                .map_err(|e| e.to_string())?;

            println!("\r\x1b[1;36m=== Tachyon Interactive Timeline ===\x1b[0m\r");
            println!("\rUse \x1b[33m↑/↓\x1b[0m to browse, \x1b[32m[Enter]\x1b[0m view commit diff, \x1b[31m[q/Esc]\x1b[0m exit\r\n");

            let visible_range = 15;
            let start = if cursor_idx >= visible_range {
                cursor_idx - visible_range + 1
            } else {
                0
            };
            let end = (start + visible_range).min(commits.len());

            for idx in start..end {
                let c = &commits[idx];
                let is_hovered = idx == cursor_idx;
                let pointer = if is_hovered { "\x1b[1;36m>\x1b[0m" } else { " " };
                let graph_node = "\x1b[35m*\x1b[0m";

                if is_hovered {
                    println!(
                        "\r{} {} \x1b[1;7m \x1b[33m{}\x1b[0m \x1b[1;37m{:28}\x1b[0m \x1b[90m{:12}\x1b[0m {} \x1b[0m\r",
                        pointer, graph_node, c.oid.short(), truncate(&c.title, 28), c.author, c.date
                    );
                } else {
                    println!(
                        "\r{} {} \x1b[33m{}\x1b[0m \x1b[1;37m{:28}\x1b[0m \x1b[90m{:12}\x1b[0m {}\r",
                        pointer, graph_node, c.oid.short(), truncate(&c.title, 28), c.author, c.date
                    );
                }
            }

            println!("\r\n\x1b[90mCommit {} of {}\x1b[0m\r", cursor_idx + 1, commits.len());
            stdout().flush().map_err(|e| e.to_string())?;

            let key = read_key()?;
            match key.code {
                KeyCode::Up => {
                    if cursor_idx > 0 {
                        cursor_idx -= 1;
                    } else {
                        cursor_idx = commits.len().saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if cursor_idx + 1 < commits.len() {
                        cursor_idx += 1;
                    } else {
                        cursor_idx = 0;
                    }
                }
                KeyCode::Enter => {
                    let selected_oid = commits[cursor_idx].oid;
                    Self::view_commit_diff(repo, &selected_oid)?;
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn view_commit_diff(repo: &Repository, commit_oid: &Oid) -> Result<(), String> {
        let commit = repo.objects.parse_commit(commit_oid)?;
        let commit_files = TreeBuilder::read_tree_recursive(&repo.objects, &commit.tree, "")?;

        let parent_files = if !commit.parents.is_empty() {
            let parent_commit = repo.objects.parse_commit(&commit.parents[0])?;
            TreeBuilder::read_tree_recursive(&repo.objects, &parent_commit.tree, "")?
        } else {
            BTreeMap::new()
        };

        let mut diff_output = String::new();
        diff_output.push_str(&format!("\x1b[1;33mcommit {}\x1b[0m\n", commit_oid.to_hex()));
        diff_output.push_str(&format!("Author: {}\n", commit.author));
        diff_output.push_str(&format!("\n    {}\n\n", commit.message));

        for (path, (_, new_blob_oid)) in &commit_files {
            let old_blob_oid = parent_files.get(path).map(|(_, oid)| *oid);
            let old_text = if let Some(oid) = old_blob_oid {
                let obj = repo.objects.read(&oid)?;
                String::from_utf8_lossy(&obj.content).to_string()
            } else {
                String::new()
            };

            let new_obj = repo.objects.read(new_blob_oid)?;
            let new_text = String::from_utf8_lossy(&new_obj.content).to_string();

            if old_text != new_text {
                let (patch, _, _) = DiffEngine::diff_text(path, path, &old_text, &new_text, true);
                diff_output.push_str(&patch);
                diff_output.push('\n');
            }
        }

        // Display scrollable diff
        let lines: Vec<&str> = diff_output.lines().collect();
        let mut scroll_offset = 0;
        let page_size = 22;

        loop {
            execute!(stdout(), cursor::MoveTo(0, 0), terminal::Clear(ClearType::All))
                .map_err(|e| e.to_string())?;

            println!("\r\x1b[1;36m=== Diff Viewer: {} ===\x1b[0m\r", commit_oid.short());
            println!("\rUse \x1b[33m↑/↓\x1b[0m to scroll, \x1b[31m[q/Esc]\x1b[0m return to timeline\r\n");

            let end = (scroll_offset + page_size).min(lines.len());
            for idx in scroll_offset..end {
                println!("\r{}\r", lines[idx]);
            }

            println!("\r\n\x1b[90mLine {}-{} of {}\x1b[0m\r", scroll_offset + 1, end, lines.len());
            stdout().flush().map_err(|e| e.to_string())?;

            let key = read_key()?;
            match key.code {
                KeyCode::Up => {
                    if scroll_offset > 0 {
                        scroll_offset -= 1;
                    }
                }
                KeyCode::Down => {
                    if scroll_offset + page_size < lines.len() {
                        scroll_offset += 1;
                    }
                }
                KeyCode::PageUp => {
                    scroll_offset = scroll_offset.saturating_sub(page_size);
                }
                KeyCode::PageDown => {
                    if scroll_offset + page_size < lines.len() {
                        scroll_offset = (scroll_offset + page_size).min(lines.len().saturating_sub(page_size));
                    }
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max.saturating_sub(3)])
    } else {
        s.to_string()
    }
}
