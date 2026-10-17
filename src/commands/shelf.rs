use std::io::IsTerminal;
use crate::core::repository::Repository;
use crate::core::stash::ShelfManager;
use crate::interactive::shelf_picker::ShelfPicker;

pub enum ShelfSubcommand {
    Save(Option<String>),
    List,
    Apply(Option<usize>),
    Pop(Option<usize>),
    Drop(Option<usize>),
    Interactive,
}

pub fn execute(sub: ShelfSubcommand) -> Result<(), String> {
    let repo = Repository::find_root()?;

    match sub {
        ShelfSubcommand::Save(msg) => {
            match ShelfManager::save(&repo, msg.as_deref())? {
                Some(id) => println!("\x1b[1;32m✓\x1b[0m Saved working directory and index state to shelf \x1b[33m#{}\x1b[0m", id),
                None => println!("No local changes to shelf."),
            }
        }
        ShelfSubcommand::List => {
            let shelves = ShelfManager::list(&repo)?;
            if shelves.is_empty() {
                println!("No stashes on the shelf.");
                return Ok(());
            }
            println!("\x1b[1;36mSaved Shelves:\x1b[0m");
            for s in shelves {
                println!("  \x1b[33m#{:<2}\x1b[0m {:35} \x1b[90m({} files) {}\x1b[0m", s.id, s.message, s.files.len(), s.timestamp);
            }
        }
        ShelfSubcommand::Apply(id_opt) => {
            let id = id_opt.unwrap_or(0);
            ShelfManager::apply(&repo, id)?;
            println!("\x1b[1;32m✓\x1b[0m Applied shelf \x1b[33m#{}\x1b[0m", id);
        }
        ShelfSubcommand::Pop(id_opt) => {
            let id = id_opt.unwrap_or(0);
            ShelfManager::pop(&repo, id)?;
            println!("\x1b[1;32m✓\x1b[0m Popped shelf \x1b[33m#{}\x1b[0m", id);
        }
        ShelfSubcommand::Drop(id_opt) => {
            let id = id_opt.unwrap_or(0);
            ShelfManager::drop(&repo, id)?;
            println!("\x1b[1;32m✓\x1b[0m Dropped shelf \x1b[33m#{}\x1b[0m", id);
        }
        ShelfSubcommand::Interactive => {
            if std::io::stdout().is_terminal() {
                let shelves = ShelfManager::list(&repo)?;
                if shelves.is_empty() {
                    println!("No stashes on the shelf.");
                    return Ok(());
                }
                match ShelfPicker::select_action(&shelves)? {
                    crate::interactive::shelf_picker::ShelfAction::Apply(id) => {
                        ShelfManager::apply(&repo, id)?;
                        println!("\x1b[1;32m✓\x1b[0m Applied shelf \x1b[33m#{}\x1b[0m", id);
                    }
                    crate::interactive::shelf_picker::ShelfAction::Pop(id) => {
                        ShelfManager::pop(&repo, id)?;
                        println!("\x1b[1;32m✓\x1b[0m Popped shelf \x1b[33m#{}\x1b[0m", id);
                    }
                    crate::interactive::shelf_picker::ShelfAction::Drop(id) => {
                        ShelfManager::drop(&repo, id)?;
                        println!("\x1b[1;32m✓\x1b[0m Dropped shelf \x1b[33m#{}\x1b[0m", id);
                    }
                    crate::interactive::shelf_picker::ShelfAction::Cancel => {
                        println!("\x1b[90mShelf action cancelled.\x1b[0m");
                    }
                }
            } else {
                execute(ShelfSubcommand::List)?;
            }
        }
    }

    Ok(())
}
