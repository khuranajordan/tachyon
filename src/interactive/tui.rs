use std::io::stdout;
use crossterm::{
    cursor,
    event::{self, Event, KeyEvent},
    execute,
    terminal::{self, ClearType},
};

pub struct TerminalGuard;

impl TerminalGuard {
    pub fn enter_raw() -> Result<Self, String> {
        terminal::enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {}", e))?;
        execute!(stdout(), cursor::Hide).map_err(|e| e.to_string())?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), cursor::Show);
        let _ = terminal::disable_raw_mode();
        println!("\r");
    }
}

pub fn read_key() -> Result<KeyEvent, String> {
    loop {
        if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
            return Ok(key);
        }
    }
}

pub fn clear_screen() {
    let _ = execute!(stdout(), terminal::Clear(ClearType::All), cursor::MoveTo(0, 0));
}
