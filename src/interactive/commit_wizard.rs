use std::io::{stdin, stdout, Write};
use crossterm::{
    cursor,
    event::KeyCode,
    execute,
    terminal::{self, ClearType},
};
use super::tui::{read_key, TerminalGuard};

pub struct CommitWizard;

const COMMIT_TYPES: &[(&str, &str)] = &[
    ("feat", "A new feature or capability"),
    ("fix", "A bug fix"),
    ("perf", "A code change that improves performance"),
    ("refactor", "A code change that neither fixes a bug nor adds a feature"),
    ("docs", "Documentation only changes"),
    ("chore", "Changes to the build process or auxiliary tools"),
    ("test", "Adding missing tests or correcting existing tests"),
    ("custom", "Freeform commit message without prefix"),
];

impl CommitWizard {
    pub fn prompt_commit_message() -> Result<Option<String>, String> {
        let _guard = TerminalGuard::enter_raw()?;
        let mut type_idx = 0;

        // Step 1: Select commit type
        loop {
            execute!(stdout(), cursor::MoveTo(0, 0), terminal::Clear(ClearType::All))
                .map_err(|e| e.to_string())?;

            println!("\r\x1b[1;36m=== Tachyon Commit Wizard (Step 1/2: Type) ===\x1b[0m\r");
            println!("\rSelect conventional commit type using \x1b[33m↑/↓\x1b[0m, confirm with \x1b[32m[Enter]\x1b[0m, \x1b[31m[q/Esc]\x1b[0m to cancel:\r\n");

            for (idx, (c_type, desc)) in COMMIT_TYPES.iter().enumerate() {
                let is_hovered = idx == type_idx;
                let pointer = if is_hovered { "\x1b[1;36m>\x1b[0m" } else { " " };

                if is_hovered {
                    println!("\r{} \x1b[1;32m{:10}\x1b[0m \x1b[1;37m{}\x1b[0m\r", pointer, c_type, desc);
                } else {
                    println!("\r{} \x1b[36m{:10}\x1b[0m \x1b[90m{}\x1b[0m\r", pointer, c_type, desc);
                }
            }

            stdout().flush().map_err(|e| e.to_string())?;

            let key = read_key()?;
            match key.code {
                KeyCode::Up => {
                    if type_idx > 0 {
                        type_idx -= 1;
                    } else {
                        type_idx = COMMIT_TYPES.len().saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if type_idx + 1 < COMMIT_TYPES.len() {
                        type_idx += 1;
                    } else {
                        type_idx = 0;
                    }
                }
                KeyCode::Enter => {
                    break;
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    return Ok(None);
                }
                _ => {}
            }
        }

        // Drop raw mode guard before line input
        drop(_guard);

        let selected_type = COMMIT_TYPES[type_idx].0;

        print!("\r\n\x1b[1;32mCommit summary:\x1b[0m ");
        stdout().flush().map_err(|e| e.to_string())?;

        let mut summary = String::new();
        stdin().read_line(&mut summary).map_err(|e| e.to_string())?;
        let summary = summary.trim();

        if summary.is_empty() {
            return Err("Commit cancelled: summary cannot be empty".to_string());
        }

        print!("\x1b[90mExtended description (optional, press Enter to skip):\x1b[0m ");
        stdout().flush().map_err(|e| e.to_string())?;

        let mut body = String::new();
        stdin().read_line(&mut body).map_err(|e| e.to_string())?;
        let body = body.trim();

        let final_message = if selected_type == "custom" {
            if body.is_empty() {
                summary.to_string()
            } else {
                format!("{}\n\n{}", summary, body)
            }
        } else if body.is_empty() {
            format!("{}: {}", selected_type, summary)
        } else {
            format!("{}: {}\n\n{}", selected_type, summary, body)
        };

        Ok(Some(final_message))
    }
}
