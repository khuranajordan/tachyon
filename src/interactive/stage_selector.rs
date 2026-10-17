use std::io::{stdout, Write};
use crossterm::{
    cursor,
    event::KeyCode,
    execute,
    terminal::{self, ClearType},
};
use super::tui::{read_key, TerminalGuard};

pub struct SelectableFile {
    pub path: String,
    pub status: &'static str,
    pub selected: bool,
}

pub struct StageSelector;

impl StageSelector {
    pub fn select_files_interactively(mut items: Vec<SelectableFile>) -> Result<Vec<String>, String> {
        if items.is_empty() {
            return Ok(Vec::new());
        }

        let _guard = TerminalGuard::enter_raw()?;
        let mut cursor_idx = 0;

        loop {
            // Render UI
            execute!(stdout(), cursor::MoveTo(0, 0), terminal::Clear(ClearType::All))
                .map_err(|e| e.to_string())?;

            println!("\r\x1b[1;36m=== Tachyon Interactive Staging ===\x1b[0m\r");
            println!("\rUse \x1b[33m↑/↓\x1b[0m to navigate, \x1b[33m[Space]\x1b[0m to toggle, \x1b[33m[a]\x1b[0m toggle all, \x1b[32m[Enter]\x1b[0m to stage, \x1b[31m[q/Esc]\x1b[0m cancel\r\n");

            for (idx, item) in items.iter().enumerate() {
                let is_hovered = idx == cursor_idx;
                let check = if item.selected { "\x1b[32m[x]\x1b[0m" } else { "[ ]" };
                let pointer = if is_hovered { "\x1b[1;36m>\x1b[0m " } else { "  " };

                let status_badge = match item.status {
                    "modified" => "\x1b[33mmodified\x1b[0m",
                    "deleted" => "\x1b[31mdeleted \x1b[0m",
                    "untracked" => "\x1b[35muntracked\x1b[0m",
                    other => other,
                };

                if is_hovered {
                    println!("\r{}\x1b[1;7m {}  {:9}  {} \x1b[0m\r", pointer, check, item.status, item.path);
                } else {
                    println!("\r{} {}  {:9}  {}\r", pointer, check, status_badge, item.path);
                }
            }

            let selected_count = items.iter().filter(|i| i.selected).count();
            println!("\r\n\x1b[90mSelected: {} of {} files\x1b[0m\r", selected_count, items.len());
            stdout().flush().map_err(|e| e.to_string())?;

            // Handle Input
            let key = read_key()?;
            match key.code {
                KeyCode::Up => {
                    if cursor_idx > 0 {
                        cursor_idx -= 1;
                    } else {
                        cursor_idx = items.len().saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if cursor_idx + 1 < items.len() {
                        cursor_idx += 1;
                    } else {
                        cursor_idx = 0;
                    }
                }
                KeyCode::Char(' ') => {
                    if let Some(item) = items.get_mut(cursor_idx) {
                        item.selected = !item.selected;
                    }
                }
                KeyCode::Char('a') => {
                    let all_selected = items.iter().all(|i| i.selected);
                    for item in &mut items {
                        item.selected = !all_selected;
                    }
                }
                KeyCode::Enter => {
                    let staged: Vec<String> = items
                        .into_iter()
                        .filter(|i| i.selected)
                        .map(|i| i.path)
                        .collect();
                    return Ok(staged);
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    return Ok(Vec::new());
                }
                _ => {}
            }
        }
    }
}
