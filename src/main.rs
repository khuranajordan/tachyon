#![allow(dead_code)]

mod core;
mod interactive;
mod commands;

use clap::{Parser, Subcommand};
use commands::shelf::ShelfSubcommand;

#[derive(Parser)]
#[command(
    name = "tachyon",
    about = "⚡ Tachyon: A hyper-speed, intuitive, interactive version control system reconstructed from Git internals",
    version = "0.1.0",
    author = "Tachyon Core Team",
    styles = styling()
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn styling() -> clap::builder::Styles {
    use clap::builder::styling::{AnsiColor, Color, Style};
    clap::builder::Styles::styled()
        .header(Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Cyan))))
        .usage(Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Green))))
        .literal(Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::BrightYellow))))
        .placeholder(Style::new().fg_color(Some(Color::Ansi(AnsiColor::White))))
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new repository [alias: create]
    #[command(alias = "create")]
    Init {
        /// Target directory
        dir: Option<String>,
        /// Initial branch name (default: main)
        #[arg(short, long)]
        branch: Option<String>,
    },

    /// Show working tree and staging status [alias: inspect, st]
    #[command(alias = "inspect", alias = "st")]
    Status,

    /// Stage changes for next save with interactive selection [alias: add]
    #[command(alias = "add")]
    Stage {
        /// Files to stage (omit to open interactive selection)
        paths: Vec<String>,
        /// Stage all tracked and untracked changes
        #[arg(short, long)]
        all: bool,
        /// Force interactive file selection mode
        #[arg(short, long)]
        interactive: bool,
    },

    /// Unstage changes from staging area [alias: reset]
    #[command(alias = "reset")]
    Unstage {
        /// Files to unstage (omit to unstage everything)
        paths: Vec<String>,
        /// Unstage all staged files
        #[arg(short, long)]
        all: bool,
    },

    /// Record changes to the repository with interactive prompt [alias: commit, ci]
    #[command(alias = "commit", alias = "ci")]
    Save {
        /// Commit message (omit to launch interactive commit wizard)
        #[arg(short, long)]
        message: Option<String>,
        /// Automatically stage modified and deleted files
        #[arg(short, long)]
        all: bool,
        /// Custom author string ("Name <email>")
        #[arg(long)]
        author: Option<String>,
    },

    /// Show changes between working tree, index, or commits [alias: diff]
    #[command(alias = "diff")]
    Changes {
        /// View changes staged in the index
        #[arg(short, long, alias = "cached")]
        staged: bool,
        /// Filter diff by file path
        #[arg(short, long)]
        path: Option<String>,
        /// Optional two commit hashes to diff
        commits: Vec<String>,
    },

    /// View commit history with interactive timeline [alias: log, timeline]
    #[command(alias = "log", alias = "timeline")]
    History {
        /// Maximum number of commits to display
        #[arg(short = 'n', long)]
        limit: Option<usize>,
        /// One commit per line format
        #[arg(long)]
        oneline: bool,
        /// Open interactive visual timeline browser
        #[arg(short, long)]
        interactive: bool,
    },

    /// List, create, or delete branches
    Branch {
        /// Name of branch to create
        name: Option<String>,
        /// Delete specified branch
        #[arg(short, long)]
        delete: Option<String>,
        /// Start point commit for new branch
        #[arg(short = 's', long)]
        start_point: Option<String>,
    },

    /// Switch branches or jump to a commit [alias: switch, checkout, co]
    #[command(alias = "switch", alias = "checkout", alias = "co")]
    Jump {
        /// Target branch or commit (omit to open interactive branch picker)
        target: Option<String>,
        /// Create a new branch and jump to it immediately
        #[arg(short = 'b', long)]
        branch: Option<String>,
        /// Force jump discarding unstaged changes
        #[arg(short, long)]
        force: bool,
    },

    /// Shelf (stash) uncommitted changes for later [alias: stash]
    #[command(alias = "stash")]
    Shelf {
        #[command(subcommand)]
        sub: Option<ShelfSub>,
    },

    /// Combine changes from another branch into current branch [alias: merge]
    #[command(alias = "merge")]
    Combine {
        /// Branch to combine/merge from
        branch: String,
    },

    /// Discard unstaged changes in working tree [alias: restore]
    #[command(alias = "restore")]
    Discard {
        /// Specific files to discard
        paths: Vec<String>,
        /// Discard all modifications in working tree
        #[arg(short, long)]
        all: bool,
    },

    /// Undo a commit by recording a new revert commit [alias: revert]
    #[command(alias = "revert")]
    Undo {
        /// Commit hash to undo
        commit: String,
    },

    /// Track line-by-line authorship of a file [alias: blame]
    #[command(alias = "blame")]
    Authors {
        /// File path to inspect
        file: String,
    },

    /// Create or list milestones (tags) [alias: tag]
    #[command(alias = "tag")]
    Milestone {
        /// Name of milestone/tag
        name: Option<String>,
        /// Target commit to tag
        #[arg(short, long)]
        commit: Option<String>,
    },

    /// Pluck a commit from another branch [alias: cherry-pick]
    #[command(alias = "cherry-pick")]
    Pluck {
        /// Commit hash to pluck
        commit: String,
    },

    /// Synchronize local repository with remote changes [alias: pull]
    #[command(alias = "pull")]
    Sync {
        /// Remote repository name (default: origin)
        remote: Option<String>,
        /// Branch name
        branch: Option<String>,
        /// Rebase local commits on top of remote branch
        #[arg(short, long)]
        rebase: bool,
    },

    /// Publish local commits to remote repository [alias: push]
    #[command(alias = "push")]
    Publish {
        /// Remote repository name (default: origin)
        remote: Option<String>,
        /// Branch name
        branch: Option<String>,
        /// Set upstream tracking reference
        #[arg(short = 'u', long)]
        set_upstream: bool,
        /// Force push with lease
        #[arg(short, long)]
        force: bool,
    },

    /// Download objects and refs from remote repository [alias: fetch, retrieve]
    #[command(alias = "fetch", alias = "retrieve")]
    Download {
        /// Remote repository name (default: origin)
        remote: Option<String>,
        /// Fetch from all remotes
        #[arg(short, long)]
        all: bool,
        /// Remove remote-tracking references that no longer exist on remote
        #[arg(short, long)]
        prune: bool,
    },

    /// Benchmark Tachyon speed against standard Git [alias: benchmark]
    #[command(alias = "benchmark")]
    Bench,
}

#[derive(Subcommand)]
enum ShelfSub {
    /// Save current working state to shelf
    Save {
        /// Optional message for this shelf item
        #[arg(short, long)]
        message: Option<String>,
    },
    /// List all stashes on shelf
    List,
    /// Apply changes from a shelf entry
    Apply {
        /// Shelf ID (default: 0)
        id: Option<usize>,
    },
    /// Apply and remove a shelf entry
    Pop {
        /// Shelf ID (default: 0)
        id: Option<usize>,
    },
    /// Drop a shelf entry
    Drop {
        /// Shelf ID (default: 0)
        id: Option<usize>,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { dir, branch } => commands::init::execute(dir, branch),
        Commands::Status => commands::status::execute(),
        Commands::Stage { paths, all, interactive } => commands::stage::execute(paths, all, interactive),
        Commands::Unstage { paths, all } => commands::unstage::execute(paths, all),
        Commands::Save { message, all, author } => commands::save::execute(message, all, author),
        Commands::Changes { staged, path, commits } => commands::changes::execute(staged, path, commits),
        Commands::History { limit, oneline, interactive } => commands::history::execute(limit, oneline, interactive),
        Commands::Branch { name, delete, start_point } => commands::branch::execute(name, delete, None, start_point),
        Commands::Jump { target, branch, force } => commands::jump::execute(target, branch, force),
        Commands::Shelf { sub } => match sub {
            Some(ShelfSub::Save { message }) => commands::shelf::execute(ShelfSubcommand::Save(message)),
            Some(ShelfSub::List) => commands::shelf::execute(ShelfSubcommand::List),
            Some(ShelfSub::Apply { id }) => commands::shelf::execute(ShelfSubcommand::Apply(id)),
            Some(ShelfSub::Pop { id }) => commands::shelf::execute(ShelfSubcommand::Pop(id)),
            Some(ShelfSub::Drop { id }) => commands::shelf::execute(ShelfSubcommand::Drop(id)),
            None => commands::shelf::execute(ShelfSubcommand::Interactive),
        },
        Commands::Combine { branch } => commands::combine::execute(branch),
        Commands::Discard { paths, all } => commands::discard::execute(paths, all),
        Commands::Undo { commit } => commands::undo::execute(commit),
        Commands::Authors { file } => commands::authors::execute(file),
        Commands::Milestone { name, commit } => commands::milestone::execute(name, commit),
        Commands::Pluck { commit } => commands::pluck::execute(commit),
        Commands::Sync { remote, branch, rebase } => commands::sync::execute(remote, branch, rebase),
        Commands::Publish { remote, branch, set_upstream, force } => commands::publish::execute(remote, branch, set_upstream, force),
        Commands::Download { remote, all, prune } => commands::fetch::execute(remote, all, prune),
        Commands::Bench => commands::bench::execute(),
    };

    if let Err(e) = result {
        eprintln!("\x1b[1;31merror:\x1b[0m {}", e);
        std::process::exit(1);
    }
}
