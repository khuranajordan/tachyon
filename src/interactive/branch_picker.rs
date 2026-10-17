use std::io::{stdout, Write};
use crossterm::{
    cursor,
    event::KeyCode,
    execute,
    terminal::{self, ClearType},
};
use super::tui::{read_key, TerminalGuard};
use crate::core::hash::Oid;

pub struct BranchPicker;

impl BranchPicker {
    pub fn select_branch(branches: &[(String, Oid, bool)]) -> Result<Option<String>, String> {
        if branches.is_empty() {
            return Ok(None);
        }

        let _guard = TerminalGuard::enter_raw()?;
        let mut cursor_idx = branches.iter().position(|(_, _, is_curr)| *is_curr).unwrap_or(0);

        loop {
            execute!(stdout(), cursor::MoveTo(0, 0), terminal::Clear(ClearType::All))
                .map_err(|e| e.to_string())?;

            println!("\r\x1b[1;36m=== Tachyon Branch Switcher ===\x1b[0m\r");
            println!("\rSelect target branch with \x1b[33m↑/↓\x1b[0m, confirm with \x1b[32m[Enter]\x1b[0m, \x1b[31m[q/Esc]\x1b[0m to cancel:\r\n");

            for (idx, (name, oid, is_current)) in branches.iter().enumerate() {
                let is_hovered = idx == cursor_idx;
                let pointer = if is_hovered { "\x1b[1;36m>\x1b[0m " } else { "  " };
                let current_badge = if *is_current { "\x1b[32m(current)\x1b[0m" } else { "" };

                if is_hovered {
                    println!("\r{}\x1b[1;7m {:20} \x1b[90m{}\x1b[0m {} \x1b[0m\r", pointer, name, oid.short(), current_badge);
                } else {
                    let name_color = if *is_current { "\x1b[1;32m" } else { "\x1b[0m" };
                    println!("\r{} {}{:20}\x1b[0m \x1b[90m{}\x1b[0m {}\r", pointer, name_color, name, oid.short(), current_badge);
                }
            }

            stdout().flush().map_err(|e| e.to_string())?;

            let key = read_key()?;
            match key.code {
                KeyCode::Up => {
                    if cursor_idx > 0 {
                        cursor_idx -= 1;
                    } else {
                        cursor_idx = branches.len().saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if cursor_idx + 1 < branches.len() {
                        cursor_idx += 1;
                    } else {
                        cursor_idx = 0;
                    }
                }
                KeyCode::Enter => {
                    return Ok(Some(branches[cursor_idx].0.clone()));
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    return Ok(None);
                }
                _ => {}
            }
        }
    }
}
