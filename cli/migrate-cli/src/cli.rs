//! The clap inbound adapter: the `accelerator-migrate` command-line surface.
//!
//! `--skip`/`--unskip`/`--unapply` each short-circuit with their own exit
//! before any migration logic runs; `--list` and `--decisions-file` fall
//! through to a run.

use std::path::PathBuf;

use clap::Parser;

/// The `accelerator-migrate` command-line surface.
#[derive(Parser)]
#[command(
    name = "accelerator-migrate",
    disable_version_flag = true,
    after_help = "Environment:\n  ACCELERATOR_MIGRATE_DECISIONS_FILE=<path>  Same as --decisions-file.\n  ACCELERATOR_MIGRATE_FORCE=1                Bypass the dirty-tree pre-flight."
)]
pub struct Cli {
    /// Mark migration <ID> skipped; do not run it.
    #[arg(long, value_name = "id")]
    pub skip: Option<String>,
    /// Remove migration <ID> from the skip list.
    #[arg(long, value_name = "id")]
    pub unskip: Option<String>,
    /// Remove migration <ID> from the applied ledger so a half-applied
    /// migration can be re-run.
    #[arg(long, value_name = "id")]
    pub unapply: Option<String>,
    /// Dry-emit pending interactive transformations, one tab-delimited line
    /// each, then exit without mutating anything.
    #[arg(long)]
    pub list: bool,
    /// Scripted decisions for interactive migrations, one per line: accept |
    /// skip | edit <value>.
    #[arg(long, value_name = "path")]
    pub decisions_file: Option<PathBuf>,
    /// Print the `SessionStart` discoverability advisory, then exit. Not
    /// part of the flag-for-flag migration surface — a hook-only entry
    /// point folded into this binary's single dispatch token.
    #[arg(long)]
    pub discoverability_hook: bool,
    /// Rendering for `--discoverability-hook`: the `SessionStart` hook
    /// envelope.
    #[arg(long, value_enum)]
    pub format: Option<Format>,
    /// On an adapter failure, degrade to a warning and exit 0 rather than
    /// exiting non-zero. Forwarded for the launcher's own external-dispatch
    /// resolution — `--discoverability-hook`'s own handler never fails, so
    /// the flag has no further effect here.
    #[arg(long)]
    pub fail_safe: bool,
}

/// How `--discoverability-hook` renders its output. `Hook` is the only
/// rendering today — carried as a flag (rather than always assumed) so a
/// future plain-text rendering has somewhere to attach without a breaking
/// CLI change.
#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    /// The `SessionStart` `additionalContext`/`systemMessage` JSON
    /// envelope.
    Hook,
}
