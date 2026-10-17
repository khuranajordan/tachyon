use std::io::{stdout, Write};
use crossterm::{
    cursor,
    event::KeyCode,
    execute,
    terminal::{self, ClearType},
};
use super::tui::{read_key, TerminalGuard};
use crate::core::stash::ShelfEntry;

pub enum ShelfAction {
    Apply(usize),
    Pop(usize),
    Drop(usize),
    Cancel,
}

pub struct ShelfPicker;

impl ShelfPicker {
    pub fn select_action(shelves: &[ShelfEntry]) -> Result<ShelfAction, String> {
        if shelves.is_empty() {
            println!("No stashes on the shelf.");
            return Ok(ShelfAction::Cancel);
        }

        let _guard = TerminalGuard::enter_raw()?;
        let mut cursor_idx = 0;

        loop {
            execute!(stdout(), cursor::MoveTo(0, 0), terminal::Clear(ClearType::All))
                .map_err(|e| e.to_string())?;

            println!("\r\x1b[1;36m=== Tachyon Shelf (Stash Manager) ===\x1b[0m\r");
            println!("\rSelect item with \x1b[33m↑/↓\x1b[0m, \x1b[32m[a]\x1b[0m Apply, \x1b[36m[p/Enter]\x1b[0m Pop, \x1b[31m[d]\x1b[0m Drop, \x1b[90m[q/Esc]\x1b[0m Cancel:\r\n");

            for (idx, shelf) in shelves.iter().enumerate() {
                let is_hovered = idx == cursor_idx;
                let pointer = if is_hovered { "\x1b[1;36m>\x1b[0m " } else { "  " };

                if is_hovered {
                    println!(
                        "\r{}\x1b[1;7m [#{}] {:30} \x1b[90m({} files)\x1b[0m {} \x1b[0m\r",
                        pointer, shelf.id, shelf.message, shelf.files.len(), shelf.timestamp
                    );
                } else {
                    println!(
                        "\r{} \x1b[33m[#{}]\x1b[0m {:30} \x1b[90m({} files)\x1b[0m {}\r",
                        pointer, shelf.id, shelf.message, shelf.files.len(), shelf.timestamp
                    );
                }
            }

            stdout().flush().map_err(|e| e.to_string())?;

            let key = read_key()?;
            let target_id = shelves[cursor_idx].id;

            match key.code {
                KeyCode::Up => {
                    if cursor_idx > 0 {
                        cursor_idx -= 1;
                    } else {
                        cursor_idx = shelves.len().saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if cursor_idx + 1 < shelves.len() {
                        cursor_idx += 1;
                    } else {
                        cursor_idx = 0;
                    }
                }
                KeyCode::Char('a') => {
                    return Ok(ShelfAction::Apply(target_id));
                }
                KeyCode::Char('p') | KeyCode::Enter => {
                    return Ok(ShelfAction::Pop(target_id));
                }
                KeyCode::Char('d') => {
                    return Ok(ShelfAction::Drop(target_id));
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    return Ok(ShelfAction::Cancel);
                }
                _ => {}
            }
        }
    }
}
