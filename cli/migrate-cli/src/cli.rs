//! The clap inbound adapter: the `accelerator-migrate` command-line surface.
//!
//! Flat — no subcommand — matching bash's `run-migrations.sh` flag shape:
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
}
