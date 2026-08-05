//! The clap inbound adapter: the `accelerator-vcs` command-line surface.

use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;

/// The `accelerator-vcs` command-line surface.
#[derive(Parser)]
#[command(name = "accelerator-vcs", disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Report the checkout's VCS mode and any workspace/worktree boundary,
    /// for the `SessionStart` hook.
    Detect {
        /// Rendering: the `SessionStart` `hook` envelope.
        #[arg(long, value_enum)]
        format: Option<Format>,
        /// Also carry the VCS command-reference cheat sheet, matching the
        /// retired shell hook's always-on prose. Without this flag, output
        /// is structured-only: the boundary block if one exists, nothing
        /// otherwise.
        #[arg(long)]
        descriptive: bool,
        /// On an adapter failure, degrade to a warning and exit 0 rather
        /// than exiting non-zero.
        #[arg(long)]
        fail_safe: bool,
    },
}

/// How `vcs detect` renders its output. `Hook` is the only rendering today —
/// carried as a flag (rather than always assumed) so a future plain-text
/// rendering has somewhere to attach without a breaking CLI change.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// The `SessionStart` `additionalContext` JSON envelope.
    Hook,
}
