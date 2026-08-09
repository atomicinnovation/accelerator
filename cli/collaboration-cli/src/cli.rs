//! The clap inbound adapter: the `accelerator-collaboration` command-line
//! surface.

use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;

/// The `accelerator-collaboration` command-line surface.
#[derive(Parser)]
#[command(name = "accelerator-collaboration", disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Pull-request operations.
    Pr {
        #[command(subcommand)]
        action: PrAction,
    },
}

#[derive(Subcommand)]
pub enum PrAction {
    /// Resolve a PR's base (upstream) owner/repo; prints "<owner>/<repo>".
    BaseRepo { pull_number: u64 },
    /// Update a PR's body from a file.
    UpdateBody {
        pull_number: u64,
        #[arg(long)]
        body_file: PathBuf,
    },
}
