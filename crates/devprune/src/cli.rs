use std::io::IsTerminal;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "devprune",
    about = "Reclaim disk space by removing developer build artifacts",
    version
)]
pub struct Cli {
    /// Directories to scan (default: current directory)
    #[arg(value_name = "PATHS")]
    pub paths: Vec<PathBuf>,

    /// Scan the home directory
    #[arg(long, conflicts_with_all = ["all", "paths"])]
    pub home: bool,

    /// Scan from the filesystem root
    #[arg(long, conflicts_with_all = ["home", "paths"])]
    pub all: bool,

    /// Force TUI mode even when stdout is not a terminal
    #[arg(long, conflicts_with = "no_tui")]
    pub tui: bool,

    /// Disable TUI and run in headless mode
    #[arg(long)]
    pub no_tui: bool,

    /// Report what would be deleted without actually deleting anything
    #[arg(long)]
    pub dry_run: bool,

    /// Output results as JSON
    #[arg(long)]
    pub json: bool,

    /// Automatically select all artifacts and delete without prompting (requires --yes)
    #[arg(long)]
    pub auto: bool,

    /// Confirm automatic deletion (required with --auto)
    #[arg(long, requires = "auto")]
    pub yes: bool,

    /// Only include artifacts from the given categories (repeatable)
    #[arg(long = "category", value_name = "CAT")]
    pub categories: Vec<String>,

    /// Minimum artifact size to report (e.g. 10MB, 1GiB)
    #[arg(long, value_name = "SIZE")]
    pub min_size: Option<String>,

    /// Maximum safety level to include: safe, cautious, or risky
    #[arg(long, value_name = "LEVEL")]
    pub safety: Option<String>,

    /// Glob patterns for paths to exclude from scanning (repeatable)
    #[arg(long = "exclude", value_name = "GLOB")]
    pub excludes: Vec<String>,

    /// Number of worker threads to use
    #[arg(long, value_name = "N")]
    pub threads: Option<usize>,

    /// Maximum directory depth to scan
    #[arg(long, value_name = "N")]
    pub max_depth: Option<usize>,

    /// Allow scanning across filesystem boundaries
    #[arg(long)]
    pub cross_device: bool,

    /// Increase verbosity (can be repeated)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress all non-error output
    #[arg(short, long)]
    pub quiet: bool,

    #[command(subcommand)]
    pub trash: Option<TrashCommand>,
}

impl Cli {
    /// Returns the effective list of paths to scan based on the flags provided.
    pub fn effective_paths(&self) -> Vec<PathBuf> {
        if self.home {
            if let Some(home) = dirs::home_dir() {
                return vec![home];
            }
        }
        if self.all {
            return vec![PathBuf::from("/")];
        }
        if !self.paths.is_empty() {
            return self.paths.clone();
        }
        vec![PathBuf::from(".")]
    }

    /// Returns true if the TUI should be launched.
    ///
    /// Priority:
    /// - `--tui` forces TUI on
    /// - `--no-tui`, `--json`, `--auto`, or `--dry-run` forces TUI off
    /// - Otherwise, fall back to whether stdout is a terminal
    pub fn use_tui(&self) -> bool {
        if self.tui {
            return true;
        }
        if self.no_tui || self.json || self.auto || self.dry_run {
            return false;
        }
        std::io::stdout().is_terminal()
    }
}

#[derive(Subcommand, Debug)]
pub enum TrashCommand {
    #[command(name = "trash")]
    Trash {
        #[command(subcommand)]
        action: TrashAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum TrashAction {
    /// List items currently in the trash
    List,
    /// Restore a trashed item by its ID
    Restore { id: String },
    /// Permanently delete trash items
    Purge {
        /// Only purge items older than this duration (e.g. 7d, 30d)
        #[arg(long, value_name = "DURATION")]
        older_than: Option<String>,
    },
}
